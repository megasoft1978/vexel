#include "reference.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define EXPAND_MAX_BYTES (16U * 1024U * 1024U)
#define EXPAND_MAX_IDS 65536U
#define EXPAND_ID_TABLE 131072U
#define EXPAND_MAX_DEPTH 16U
#define EXPAND_MAX_INSTANCES 65536U

typedef struct reference_id { svg_xml_span id; size_t token; } reference_id;
typedef struct expander {
    base_bytes src;
    svg_xml_token *tokens;
    size_t count;
    reference_id *ids;
    size_t *id_slots;
    size_t id_count;
    uint8_t *out;
    size_t len, cap, instances;
    size_t stack[EXPAND_MAX_DEPTH];
    size_t stack_len;
    size_t first_use_offset;
    bool have_first_use;
    svg_reference_source_map *map;
    svg_shape_diagnostic *diag;
} expander;

static bool eq(base_bytes s, svg_xml_span v, const char *z)
{ size_t n=strlen(z); return v.offset<=s.len && v.length==n && n<=s.len-v.offset && memcmp(s.ptr+v.offset,z,n)==0; }
static bool attr(expander *e,size_t at,const char *local,const char *prefix,svg_xml_span *v)
{
    for(size_t i=at+1U;i<e->count&&e->tokens[i].kind==SVG_XML_TOKEN_ATTRIBUTE;i++)
        if(eq(e->src,e->tokens[i].local_name,local) &&
           ((prefix==NULL&&e->tokens[i].prefix.length==0U)||(prefix&&eq(e->src,e->tokens[i].prefix,prefix)))) { *v=e->tokens[i].value; return true; }
    return false;
}
static size_t close_token(expander *e,size_t at)
{
    if(e->tokens[at].kind==SVG_XML_TOKEN_EMPTY_ELEMENT)return at;
    for(size_t i=at+1U;i<e->count;i++) if(e->tokens[i].kind==SVG_XML_TOKEN_END_ELEMENT&&e->tokens[i].depth==e->tokens[at].depth)return i;
    return at;
}
static bool append(expander *e,const uint8_t *p,size_t n)
{
    if(n>EXPAND_MAX_BYTES-e->len)return false;
    if(e->len+n>e->cap){size_t c=e->cap?e->cap:256U;while(c<e->len+n){if(c>EXPAND_MAX_BYTES/2U){c=EXPAND_MAX_BYTES;break;}c*=2U;}uint8_t *q=realloc(e->out,c);if(!q)return false;e->out=q;e->cap=c;}
    if(n)memcpy(e->out+e->len,p,n);e->len+=n;return true;
}
static bool literal(expander *e,const char *s){return append(e,(const uint8_t *)s,strlen(s));}
static bool source_map_add(expander *e, size_t output_offset, size_t length,
                           size_t source_offset)
{
    if (e->map == NULL || length == 0U) return true;
    if (e->map->count != 0U) {
        svg_reference_source_map_entry *last = &e->map->entries[e->map->count - 1U];
        if (last->generated_offset + last->length == output_offset &&
            last->source_offset + last->length == source_offset) {
            last->length += length;
            return true;
        }
    }
    if (e->map->count >= 1048576U || e->map->count + 1U > SIZE_MAX / sizeof(*e->map->entries)) return false;
    size_t next = e->map->count + 1U;
    svg_reference_source_map_entry *grown = realloc(e->map->entries, next * sizeof(*e->map->entries));
    if (grown == NULL) return false;
    e->map->entries = grown;
    e->map->entries[e->map->count++] = (svg_reference_source_map_entry){output_offset, length, source_offset};
    return true;
}
static bool source_span(expander *e,svg_xml_span s)
{
    if(s.offset>e->src.len||s.length>e->src.len-s.offset)return false;
    size_t start=e->len;
    return append(e,e->src.ptr+s.offset,s.length)&&source_map_add(e,start,s.length,s.offset);
}
static void error_at(expander *e,svg_shape_error kind,size_t offset){if(e->diag)*e->diag=(svg_shape_diagnostic){kind,offset};}
static size_t find_id(expander *e,svg_xml_span id)
{
    uint64_t hash=UINT64_C(1469598103934665603);
    for(size_t i=0U;i<id.length;i++){hash^=e->src.ptr[id.offset+i];hash*=UINT64_C(1099511628211);}
    size_t slot=(size_t)hash&(EXPAND_ID_TABLE-1U);
    while(e->id_slots[slot]!=0U){reference_id *entry=&e->ids[e->id_slots[slot]-1U];if(entry->id.length==id.length&&memcmp(e->src.ptr+entry->id.offset,e->src.ptr+id.offset,id.length)==0)return entry->token;slot=(slot+1U)&(EXPAND_ID_TABLE-1U);}
    return SIZE_MAX;
}
static bool number_text(expander *e,svg_xml_span s,double *v,double fallback)
{
    if(s.length==0){*v=fallback;return true;} char buf[96];if(s.length>=sizeof(buf))return false;memcpy(buf,e->src.ptr+s.offset,s.length);buf[s.length]=0;char *end=NULL;*v=strtod(buf,&end);return end==buf+s.length&&isfinite(*v)&&fabs(*v)<=1000000.0;
}
static bool symbol_matrix(expander *e,size_t target,size_t use,char *buf,size_t cap)
{
    svg_xml_span vb={0},wspan={0},hspan={0},xspan={0},yspan={0},par={0};
    if(!attr(e,target,"viewBox",NULL,&vb)||!attr(e,use,"width",NULL,&wspan)||!attr(e,use,"height",NULL,&hspan))return false;
    if(attr(e,use,"preserveAspectRatio",NULL,&par)&&!eq(e->src,par,"xMidYMid meet"))return false;
    double v[4],w,h,x,y;if(!number_text(e,wspan,&w,0)||!number_text(e,hspan,&h,0)||!number_text(e,xspan,&x,0)||!number_text(e,yspan,&y,0))return false;
    char raw[256];if(vb.length>=sizeof(raw))return false;memcpy(raw,e->src.ptr+vb.offset,vb.length);raw[vb.length]=0;
    char *p=raw;for(size_t i=0;i<4;i++){while(*p==' '||*p=='\t'||*p=='\r'||*p=='\n'||*p==',')p++;char *end;v[i]=strtod(p,&end);if(end==p||!isfinite(v[i]))return false;p=end;}
    while(*p==' '||*p=='\t'||*p=='\r'||*p=='\n'||*p==',')p++;if(*p||v[2]<=0||v[3]<=0||w<=0||h<=0)return false;
    if(attr(e,use,"x",NULL,&xspan)&&!number_text(e,xspan,&x,0))return false;
    if(attr(e,use,"y",NULL,&yspan)&&!number_text(e,yspan,&y,0))return false;
    double s=fmin(w/v[2],h/v[3]),tx=x+(w-v[2]*s)/2.0-v[0]*s,ty=y+(h-v[3]*s)/2.0-v[1]*s;
    int n=snprintf(buf,cap,"matrix(%.17g 0 0 %.17g %.17g %.17g)",s,s,tx,ty);return n>0&&(size_t)n<cap;
}

static bool expand_use(expander *e,size_t use,size_t use_end);

/* Copy a byte interval, recursively replacing use elements in the interval. */
static bool emit_interval(expander *e,size_t from,size_t to,size_t token_lo,size_t token_hi,bool skip_definitions)
{
    size_t cursor=from;
    for(size_t i=token_lo;i<token_hi;i++){
        svg_xml_token *t=&e->tokens[i];if(t->kind!=SVG_XML_TOKEN_START_ELEMENT&&t->kind!=SVG_XML_TOKEN_EMPTY_ELEMENT)continue;
        size_t end=close_token(e,i);
        if(skip_definitions&&t->prefix.length==0U&&(eq(e->src,t->local_name,"defs")||eq(e->src,t->local_name,"symbol"))){i=end;continue;}
        if(t->prefix.length==0U&&eq(e->src,t->local_name,"use")){
            if(t->source.offset<cursor||t->source.offset>to)continue;
            if(!source_span(e,(svg_xml_span){cursor,t->source.offset-cursor})||!expand_use(e,i,end))return false;
            size_t stop=e->tokens[end].source.offset+e->tokens[end].source.length;cursor=stop;i=end;
        }
    }
    return cursor<=to&&source_span(e,(svg_xml_span){cursor,to-cursor});
}

static bool expand_use(expander *e,size_t use,size_t use_end)
{
    if (!e->have_first_use) { e->first_use_offset=e->tokens[use].name.offset; e->have_first_use=true; }
    svg_xml_span href={0};bool plain=attr(e,use,"href",NULL,&href),xl=attr(e,use,"href","xlink",&href);
    if(plain==xl||href.length<2U||e->src.ptr[href.offset]!='#'){error_at(e,SVG_SHAPE_ERROR_UNSUPPORTED,e->tokens[use].name.offset);return false;}
    size_t target=find_id(e,(svg_xml_span){href.offset+1U,href.length-1U});
    if(target==SIZE_MAX){error_at(e,SVG_SHAPE_ERROR_UNSUPPORTED,href.offset);return false;}
    for(size_t i=0;i<e->stack_len;i++)if(e->stack[i]==target){error_at(e,SVG_SHAPE_ERROR_UNSUPPORTED,href.offset);return false;}
    if(e->stack_len>=EXPAND_MAX_DEPTH||++e->instances>EXPAND_MAX_INSTANCES){error_at(e,SVG_SHAPE_ERROR_LIMIT,e->tokens[use].name.offset);return false;}
    bool symbol=eq(e->src,e->tokens[target].local_name,"symbol");
    bool supported=symbol||eq(e->src,e->tokens[target].local_name,"g")||eq(e->src,e->tokens[target].local_name,"rect")||eq(e->src,e->tokens[target].local_name,"path")||eq(e->src,e->tokens[target].local_name,"circle")||eq(e->src,e->tokens[target].local_name,"ellipse")||eq(e->src,e->tokens[target].local_name,"line")||eq(e->src,e->tokens[target].local_name,"polygon")||eq(e->src,e->tokens[target].local_name,"polyline");
    if(!supported){error_at(e,SVG_SHAPE_ERROR_UNSUPPORTED,e->tokens[target].name.offset);return false;}
    char matrix[160];bool has_matrix=false;if(symbol){if(!symbol_matrix(e,target,use,matrix,sizeof(matrix))){svg_xml_span p={0};size_t off=attr(e,use,"preserveAspectRatio",NULL,&p)?p.offset:e->tokens[use].name.offset;error_at(e,SVG_SHAPE_ERROR_UNSUPPORTED,off);return false;}has_matrix=true;}
    svg_xml_span x={0},y={0},tr={0};bool hx=attr(e,use,"x",NULL,&x),hy=attr(e,use,"y",NULL,&y),ht=attr(e,use,"transform",NULL,&tr);
    if(!literal(e,"<g"))return false;
    for(size_t a=use+1U;a<e->count&&e->tokens[a].kind==SVG_XML_TOKEN_ATTRIBUTE;a++){
        svg_xml_token *at=&e->tokens[a];bool omit=eq(e->src,at->local_name,"href")||eq(e->src,at->local_name,"x")||eq(e->src,at->local_name,"y")||eq(e->src,at->local_name,"width")||eq(e->src,at->local_name,"height")||eq(e->src,at->local_name,"transform");
        if(!omit&&(!source_span(e,(svg_xml_span){at->source.offset-1U,1U})||!source_span(e,at->source)))return false;
    }
    if(hx||hy||has_matrix||ht){if(!literal(e," transform=\""))return false;
        if(has_matrix){if(!append(e,(const uint8_t *)matrix,strlen(matrix)))return false;}
        else {char xy[128];double xv=0,yv=0;if((hx&&!number_text(e,x,&xv,0))||(hy&&!number_text(e,y,&yv,0))){error_at(e,SVG_SHAPE_ERROR_UNSUPPORTED,x.offset);return false;}int n=snprintf(xy,sizeof(xy),"translate(%.17g %.17g)",xv,yv);if(n<0||!append(e,(const uint8_t *)xy,(size_t)n))return false;}
        if(ht&&(!literal(e," ")||!source_span(e,tr)))return false;if(!literal(e,"\""))return false;
    }
    if(!literal(e,">"))return false;
    e->stack[e->stack_len++]=target;
    size_t target_end=close_token(e,target);
    size_t content_start,target_stop;
    if(symbol){content_start=e->tokens[target].source.offset+e->tokens[target].source.length;target_stop=e->tokens[target_end].source.offset;}
    else {content_start=e->tokens[target].source.offset;target_stop=e->tokens[target_end].source.offset+e->tokens[target_end].source.length;}
    bool ok=emit_interval(e,content_start,target_stop,target+1U,target_end,false);
    e->stack_len--;
    if(!ok||!literal(e,"</g>"))return false;
    (void)use_end;return true;
}

svg_shape_error svg_reference_expand_mapped(base_bytes source,base_bytes *expanded,
                                             svg_reference_source_map *map,
                                             svg_shape_diagnostic *diagnostic)
{
    if(!expanded||!base_bytes_valid(source)){error_at(&(expander){.diag=diagnostic},SVG_SHAPE_ERROR_INVALID_ARGUMENT,0);return SVG_SHAPE_ERROR_INVALID_ARGUMENT;}
    if (map != NULL) *map = (svg_reference_source_map){0};
    if (diagnostic != NULL) *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0U};
    *expanded=(base_bytes){0};size_t cap=source.len/2U+2U;svg_xml_token *tokens=calloc(cap,sizeof(*tokens));if(!tokens){error_at(&(expander){.diag=diagnostic},SVG_SHAPE_ERROR_OOM,0);return SVG_SHAPE_ERROR_OOM;}
    svg_xml_result xr=svg_xml_tokenize(source,svg_xml_default_options(),tokens,cap);if(xr.diagnostic.category!=SVG_XML_ERROR_NONE){free(tokens);error_at(&(expander){.diag=diagnostic},SVG_SHAPE_ERROR_XML,xr.diagnostic.offset);return SVG_SHAPE_ERROR_XML;}
    svg_shape_error validation = svg_reference_validate(source, tokens, xr.token_count, diagnostic);
    if (validation != SVG_SHAPE_ERROR_NONE) { free(tokens); return validation; }
    expander e={.src=source,.tokens=tokens,.count=xr.token_count,.diag=diagnostic,.map=map};e.ids=calloc(EXPAND_MAX_IDS,sizeof(*e.ids));e.id_slots=calloc(EXPAND_ID_TABLE,sizeof(*e.id_slots));if(!e.ids||!e.id_slots){free(e.ids);free(e.id_slots);free(tokens);error_at(&e,SVG_SHAPE_ERROR_OOM,0);return SVG_SHAPE_ERROR_OOM;}
    for(size_t i=0;i<e.count;i++)if(tokens[i].kind==SVG_XML_TOKEN_START_ELEMENT||tokens[i].kind==SVG_XML_TOKEN_EMPTY_ELEMENT){svg_xml_span id;if(attr(&e,i,"id",NULL,&id)){if(e.id_count==EXPAND_MAX_IDS){error_at(&e,SVG_SHAPE_ERROR_LIMIT,id.offset);goto fail;}size_t index=e.id_count++;e.ids[index]=(reference_id){id,i};uint64_t hash=UINT64_C(1469598103934665603);for(size_t j=0U;j<id.length;j++){hash^=source.ptr[id.offset+j];hash*=UINT64_C(1099511628211);}size_t slot=(size_t)hash&(EXPAND_ID_TABLE-1U);while(e.id_slots[slot]!=0U)slot=(slot+1U)&(EXPAND_ID_TABLE-1U);e.id_slots[slot]=index+1U;}}
    /* Keep source byte-for-byte when there is no live use; validation still runs in decode. */
    bool any=false,hidden[65]={false};for(size_t i=0U;i<e.count;i++){svg_xml_token *t=&tokens[i];if(t->kind!=SVG_XML_TOKEN_START_ELEMENT&&t->kind!=SVG_XML_TOKEN_EMPTY_ELEMENT)continue;size_t d=t->depth;bool parent_hidden=d>0U&&d<65U?hidden[d-1U]:false;bool is_definition=t->prefix.length==0U&&(eq(source,t->local_name,"defs")||eq(source,t->local_name,"symbol"));hidden[d]=parent_hidden||is_definition;if(!hidden[d]&&t->prefix.length==0U&&eq(source,t->local_name,"use"))any=true;}
    if(!any){free(e.id_slots);free(e.ids);free(tokens);*expanded=source;return SVG_SHAPE_ERROR_NONE;}
    if(!emit_interval(&e,0,source.len,0,e.count,true)){if(diagnostic&&diagnostic->category==SVG_SHAPE_ERROR_NONE)error_at(&e,SVG_SHAPE_ERROR_LIMIT,e.first_use_offset);goto fail;}
    if (map != NULL) map->fallback_offset = e.first_use_offset;
    free(e.id_slots);free(e.ids);free(tokens);*expanded=(base_bytes){e.out,e.len};return SVG_SHAPE_ERROR_NONE;
fail: free(e.out);free(e.id_slots);free(e.ids);free(tokens);if(map!=NULL)svg_reference_source_map_destroy(map);return diagnostic?diagnostic->category:SVG_SHAPE_ERROR_UNSUPPORTED;
}

svg_shape_error svg_reference_expand(base_bytes source, base_bytes *expanded,
                                     svg_shape_diagnostic *diagnostic)
{
    return svg_reference_expand_mapped(source, expanded, NULL, diagnostic);
}

size_t svg_reference_map_offset(const svg_reference_source_map *map,
                                size_t generated_offset)
{
    if (map == NULL) return generated_offset;
    for (size_t i = 0U; i < map->count; ++i) {
        const svg_reference_source_map_entry *entry = &map->entries[i];
        if (generated_offset >= entry->generated_offset &&
            generated_offset - entry->generated_offset < entry->length)
            return entry->source_offset + generated_offset - entry->generated_offset;
    }
    return map->fallback_offset;
}

void svg_reference_source_map_destroy(svg_reference_source_map *map)
{
    if (map == NULL) return;
    free(map->entries);
    *map = (svg_reference_source_map){0};
}
