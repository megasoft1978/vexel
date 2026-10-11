#include "xml_tokenizer.h"
#include "xml_internal.h"
#include "xml_tokenizer_private.h"

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#define XML_MAX_NAME_BYTES 4096U

typedef struct xml_open_name { svg_xml_span span; } xml_open_name;
typedef enum xml_stream_pending_kind {
    XML_STREAM_PENDING_NONE = 0,
    XML_STREAM_PENDING_TEXT,
    XML_STREAM_PENDING_COMMENT,
    XML_STREAM_PENDING_CDATA,
    XML_STREAM_PENDING_PI,
    XML_STREAM_PENDING_TAG
} xml_stream_pending_kind;
typedef struct xml_scanner {
    base_bytes input;
    svg_xml_options options;
    svg_xml_token *tokens;
    size_t capacity;
    svg_xml_result result;
    size_t cursor;
    size_t root_count;
    size_t depth;
    size_t total_attributes;
    size_t char_cursor;
    size_t char_sequence_start;
    size_t token_storage_limit;
    xml_stream_pending_kind pending_kind;
    size_t pending_start;
    size_t pending_cursor;
    size_t pending_data;
    size_t pending_first_nonspace;
    svg_xml_span pending_name;
    uint8_t pending_quote;
    bool pending_nonspace;
    bool pending_declaration;
    bool pending_pi_data_ready;
    bool stream_start_active;
    bool stream_start_separated;
    bool stream_end_active;
    bool stream_name_active;
    bool stream_start_name_ready;
    bool stream_end_name_ready;
    unsigned stream_end_prefix_progress;
    unsigned stream_start_phase;
    unsigned char_pending_length;
    unsigned char_pending_width;
    size_t stream_start_pos;
    size_t stream_start_token_index;
    size_t stream_start_local_attributes;
    size_t stream_start_attribute_offset;
    size_t stream_start_value_offset;
    size_t stream_end_pos;
    size_t stream_name_start;
    size_t stream_name_pos;
    size_t stream_name_first_colon;
    svg_xml_span stream_end_name;
    svg_xml_span stream_end_prefix;
    svg_xml_span stream_end_local_name;
    svg_xml_span stream_start_name;
    svg_xml_span stream_start_prefix;
    svg_xml_span stream_start_local_name;
    svg_xml_span stream_attribute_name;
    svg_xml_span stream_attribute_prefix;
    svg_xml_span stream_attribute_local_name;
    uint8_t stream_start_quote;
    uint8_t char_pending[4];
    svg_xml_cancel_check_fn cancel_check;
    void *cancel_context;
    bool canceled;
    bool chars_complete;
    bool owns_tokens;
    bool stream_complete;
    xml_open_name stack[XML_MAX_DEPTH];
} xml_scanner;

struct svg_xml_token_stream { xml_scanner scanner; };

static bool cancel_requested(xml_scanner *s)
{
    if (s->cancel_check != NULL && s->cancel_check(s->cancel_context)) {
        s->canceled = true;
        return true;
    }
    return false;
}

static bool add_size(size_t a, size_t b, size_t *out)
{
    if (out == NULL || b > SIZE_MAX - a) return false;
    *out = a + b;
    return true;
}

static bool span_at(size_t start, size_t end, svg_xml_span *out)
{
    if (out == NULL || end < start) return false;
    out->offset = start;
    out->length = end - start;
    return true;
}

static uint8_t byte_at(const xml_scanner *s, size_t at)
{
    return s->input.ptr[at];
}

static bool starts(const xml_scanner *s, size_t at, const char *literal, size_t length)
{
    return at <= s->input.len && length <= s->input.len - at &&
           memcmp(s->input.ptr + at, literal, length) == 0;
}

static void fail(xml_scanner *s, svg_xml_error category, size_t offset)
{
    if (s->result.diagnostic.category == SVG_XML_ERROR_NONE) {
        s->result.diagnostic.category = category;
        s->result.diagnostic.offset = offset <= s->input.len ? offset : s->input.len;
    }
}

static bool emit(xml_scanner *s, svg_xml_token token, size_t at)
{
    size_t next = 0;
    if (!add_size(s->result.token_count, 1, &next)) {
        fail(s, SVG_XML_ERROR_OUTPUT_EXHAUSTED, at);
        return false;
    }
    if (next > s->capacity && s->owns_tokens) {
        size_t capacity = s->capacity == 0U ? 256U : s->capacity;
        while (capacity < next) {
            size_t doubled = 0U;
            if (!base_size_mul(capacity, 2U, &doubled)) break;
            capacity = doubled;
        }
        size_t maximum = s->token_storage_limit / sizeof(*s->tokens);
        if (capacity > maximum) capacity = maximum;
        size_t bytes = 0U;
        if (capacity < next || !base_size_mul(capacity, sizeof(*s->tokens), &bytes)) {
            fail(s, SVG_XML_ERROR_OUTPUT_EXHAUSTED, at);
            return false;
        }
        svg_xml_token *grown = realloc(s->tokens, bytes);
        if (grown == NULL) {
            fail(s, SVG_XML_ERROR_OUTPUT_EXHAUSTED, at);
            return false;
        }
        s->tokens = grown;
        s->capacity = capacity;
    }
    if (next > s->capacity) {
        fail(s, SVG_XML_ERROR_OUTPUT_EXHAUSTED, at);
        return false;
    }
    s->tokens[s->result.token_count] = token;
    s->result.token_count = next;
    return true;
}

static bool count_node(xml_scanner *s, size_t at)
{
    size_t next = 0;
    if (!add_size(s->result.node_count, 1, &next) || next > s->options.limits.nodes) {
        fail(s, SVG_XML_ERROR_NODE_LIMIT, at);
        return false;
    }
    s->result.node_count = next;
    return true;
}

static bool parse_name(xml_scanner *s, size_t *position, svg_xml_span *span,
                       svg_xml_span *prefix, svg_xml_span *local)
{
    size_t p = *position;
    size_t first_end = p;
    uint32_t cp = 0;
    if (!svg_xml_internal_utf8_next(s->input.ptr, s->input.len, &p, &cp) || !svg_xml_internal_name_start(cp)) {
        fail(s, SVG_XML_ERROR_INVALID_NAME, *position);
        return false;
    }
    while (!cancel_requested(s) && p < s->input.len) {
        size_t before = p;
        if (!svg_xml_internal_utf8_next(s->input.ptr, s->input.len, &p, &cp)) {
            fail(s, SVG_XML_ERROR_INVALID_UTF8, before);
            return false;
        }
        if (!svg_xml_internal_name_char(cp)) { p = before; break; }
        if (cp == ':' && first_end == *position) first_end = before;
    }
    if(byte_at(s,*position)==':' || byte_at(s,p-1)==':') {
        fail(s,SVG_XML_ERROR_INVALID_NAME,*position);
        return false;
    }
    if (p - *position > XML_MAX_NAME_BYTES) {
        fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, *position);
        return false;
    }
    (void)span_at(*position, p, span);
    if (first_end == *position) {
        prefix->offset = *position; prefix->length = 0;
        local->offset = *position; local->length = p - *position;
    } else {
        prefix->offset = *position; prefix->length = first_end - *position;
        local->offset = first_end + 1; local->length = p - (first_end + 1);
        if (local->length == 0 || memchr(s->input.ptr + local->offset, ':', local->length) != NULL) {
            fail(s, SVG_XML_ERROR_INVALID_NAME, first_end);
            return false;
        }
    }
    *position = p;
    return true;
}

static bool parse_pi_target(xml_scanner *s, size_t *position, svg_xml_span *name)
{
    size_t p = *position;
    size_t start = p;
    uint32_t cp = 0;
    if (!svg_xml_internal_utf8_next(s->input.ptr, s->input.len, &p, &cp) || !svg_xml_internal_name_start(cp)) {
        fail(s, SVG_XML_ERROR_INVALID_NAME, start);
        return false;
    }
    while (!cancel_requested(s) && p < s->input.len) {
        size_t before = p;
        if (!svg_xml_internal_utf8_next(s->input.ptr, s->input.len, &p, &cp)) {
            fail(s, SVG_XML_ERROR_INVALID_UTF8, before);
            return false;
        }
        if (!svg_xml_internal_name_char(cp)) { p = before; break; }
    }
    if (p - start > XML_MAX_NAME_BYTES) {
        fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, start);
        return false;
    }
    (void)span_at(start, p, name);
    *position = p;
    return true;
}

static bool valid_qname(xml_scanner *s, svg_xml_span prefix, svg_xml_span local)
{
    uint32_t first = 0;
    size_t at = local.offset;
    if (local.length == 0 || !svg_xml_internal_utf8_next(s->input.ptr, s->input.len, &at, &first) ||
        !svg_xml_internal_name_start(first)) {
        fail(s, SVG_XML_ERROR_INVALID_NAME, local.offset);
        return false;
    }
    if (prefix.length != 0) {
        at = prefix.offset;
        if (!svg_xml_internal_utf8_next(s->input.ptr, s->input.len, &at, &first) || !svg_xml_internal_name_start(first)) {
            fail(s, SVG_XML_ERROR_INVALID_NAME, prefix.offset);
            return false;
        }
    }
    return true;
}

static bool scan_reference(xml_scanner *s, size_t *position)
{
    size_t p = *position + 1;
    if (p >= s->input.len) { fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, *position); return false; }
    if (byte_at(s, p) == '#') {
        ++p;
        bool hex = false;
        if (p < s->input.len && byte_at(s, p) == 'x') { hex = true; ++p; }
        size_t digits = p;
        uint32_t value = 0;
        while (!cancel_requested(s) && p < s->input.len && (hex ? ((byte_at(s,p)>='0'&&byte_at(s,p)<='9') ||
               (byte_at(s,p)>='a'&&byte_at(s,p)<='f') || (byte_at(s,p)>='A'&&byte_at(s,p)<='F')) :
               (byte_at(s,p)>='0'&&byte_at(s,p)<='9'))) {
            uint8_t c=byte_at(s,p);
            uint32_t digit=(uint32_t)(c>='0'&&c<='9'?c-'0':(c>='a'&&c<='f'?c-'a'+10U:c-'A'+10U));
            if(value>0x10ffffU/(hex?16U:10U)) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,*position);return false; }
            value=value*(hex?16U:10U)+digit;
            if(value>0x10ffffU) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,*position);return false; }
            ++p;
        }
        if (p == digits || p >= s->input.len || byte_at(s,p) != ';') {
            fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, *position); return false;
        }
        if(!svg_xml_internal_char(value)) { fail(s,SVG_XML_ERROR_INVALID_CHARACTER,*position);return false; }
    } else {
        size_t start = p;
        uint32_t cp = 0;
        if (!svg_xml_internal_utf8_next(s->input.ptr, s->input.len, &p, &cp) || !svg_xml_internal_name_start(cp)) {
            fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, *position); return false;
        }
        while (!cancel_requested(s) && p < s->input.len) {
            size_t before=p;
            if (!svg_xml_internal_utf8_next(s->input.ptr,s->input.len,&p,&cp)) { fail(s,SVG_XML_ERROR_INVALID_UTF8,before); return false; }
            if (!svg_xml_internal_name_char(cp)) { p=before; break; }
        }
        if (p >= s->input.len || byte_at(s,p) != ';') { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,*position); return false; }
        size_t n=p-start;
        if (!((n==2 && memcmp(s->input.ptr+start,"lt",2)==0) ||
              (n==2 && memcmp(s->input.ptr+start,"gt",2)==0) ||
              (n==3 && memcmp(s->input.ptr+start,"amp",3)==0) ||
              (n==4 && memcmp(s->input.ptr+start,"apos",4)==0) ||
              (n==4 && memcmp(s->input.ptr+start,"quot",4)==0))) {
            fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, *position); return false;
        }
    }
    ++p;
    *position = p;
    return true;
}

static bool scan_chars(xml_scanner *s)
{
    size_t p=0;
    while (!cancel_requested(s) && p<s->input.len) {
        size_t at=p; uint32_t cp=0;
        if (!svg_xml_internal_utf8_next(s->input.ptr,s->input.len,&p,&cp)) { fail(s,SVG_XML_ERROR_INVALID_UTF8,at); return false; }
        if (!svg_xml_internal_char(cp)) { fail(s,SVG_XML_ERROR_INVALID_CHARACTER,at); return false; }
    }
    return true;
}

static bool qname_equal(xml_scanner *s, svg_xml_span a, svg_xml_span b)
{
    return a.length==b.length && a.offset<=s->input.len && b.offset<=s->input.len &&
        a.length<=s->input.len-a.offset && b.length<=s->input.len-b.offset &&
        memcmp(s->input.ptr+a.offset,s->input.ptr+b.offset,a.length)==0;
}

static bool parse_comment(xml_scanner *s)
{
    size_t start=s->cursor, p=start+4, data=p;
    while (!cancel_requested(s) && p<s->input.len && !starts(s,p,"-->",3)) {
        if (starts(s,p,"--",2)) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,p); return false; }
        ++p;
    }
    if (p>=s->input.len) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,start); return false; }
    svg_xml_token t={0}; t.kind=SVG_XML_TOKEN_COMMENT; t.depth=s->depth;
    (void)span_at(start,p+3,&t.source); (void)span_at(data,p,&t.value);
    if (!count_node(s,start) || !emit(s,t,start)) return false;
    s->cursor=p+3; return true;
}

static bool parse_pi(xml_scanner *s)
{
    size_t start=s->cursor, p=start+2; svg_xml_span name={0};
    if (!parse_pi_target(s,&p,&name)) return false;
    bool declaration=name.length==3 && memcmp(s->input.ptr+name.offset,"xml",3)==0;
    bool has_bom = starts(s, 0, "\xef\xbb\xbf", 3);
    if (declaration && start!=0 && !(has_bom && start==3)) {
        fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,start); return false;
    }
    if (!declaration && name.length==3) {
        uint8_t a=s->input.ptr[name.offset], b=s->input.ptr[name.offset+1], c=s->input.ptr[name.offset+2];
        if ((a=='x'||a=='X')&&(b=='m'||b=='M')&&(c=='l'||c=='L')) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,start); return false; }
    }
    if (p>=s->input.len || (!svg_xml_internal_whitespace(byte_at(s,p)) && !starts(s,p,"?>",2))) {
        fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,p); return false;
    }
    if(declaration && starts(s,p,"?>",2)) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,p);return false; }
    if (svg_xml_internal_whitespace(byte_at(s,p))) while (!cancel_requested(s) && p<s->input.len && svg_xml_internal_whitespace(byte_at(s,p))) ++p;
    size_t data=p;
    while (!cancel_requested(s) && p<s->input.len && !starts(s,p,"?>",2)) ++p;
    if (p>=s->input.len) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,start); return false; }
    svg_xml_token t={0}; t.kind=declaration?SVG_XML_TOKEN_DECLARATION:SVG_XML_TOKEN_PROCESSING_INSTRUCTION;
    t.name=name; t.value.offset=data; t.value.length=p-data; t.depth=s->depth;
    (void)span_at(start,p+2,&t.source);
    if (!declaration && !count_node(s,start)) return false;
    if (!emit(s,t,start)) return false;
    s->cursor=p+2; return true;
}

static bool parse_text(xml_scanner *s)
{
    size_t start=s->cursor,p=start;
    while (!cancel_requested(s) && p<s->input.len && byte_at(s,p)!='<') {
        if (starts(s,p,"]]>",3)) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,p); return false; }
        if (byte_at(s,p)=='&') { if (!scan_reference(s,&p)) return false; }
        else ++p;
    }
    if (p==start) return true;
    if (p-start>s->options.limits.text_run_bytes) { fail(s,SVG_XML_ERROR_TEXT_LIMIT,start); return false; }
    if (s->depth==0) for (size_t i=start;i<p;++i) if (!svg_xml_internal_whitespace(byte_at(s,i))) {
        fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,i); return false;
    }
    svg_xml_token t={0}; t.kind=SVG_XML_TOKEN_TEXT; t.depth=s->depth;
    (void)span_at(start,p,&t.source); t.value=t.source;
    bool nonspace=false; for(size_t i=start;i<p;++i) if(!svg_xml_internal_whitespace(byte_at(s,i))) nonspace=true;
    if ((s->depth!=0 || nonspace) && !count_node(s,start)) return false;
    if (!emit(s,t,start)) return false;
    s->cursor=p; return true;
}

static bool parse_cdata(xml_scanner *s)
{
    size_t start=s->cursor, data=start+9,p=data;
    while(!cancel_requested(s) && p<s->input.len && !starts(s,p,"]]>",3)) ++p;
    if(p>=s->input.len) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,start); return false; }
    if(s->depth==0) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,start); return false; }
    if(p-data>s->options.limits.text_run_bytes) { fail(s,SVG_XML_ERROR_TEXT_LIMIT,data); return false; }
    svg_xml_token t={0}; t.kind=SVG_XML_TOKEN_CDATA;t.depth=s->depth;
    (void)span_at(start,p+3,&t.source); (void)span_at(data,p,&t.value);
    if(!count_node(s,start)||!emit(s,t,start)) return false;
    s->cursor=p+3;return true;
}

static bool parse_end(xml_scanner *s)
{
    size_t start=s->cursor,p=start+2; svg_xml_span name={0},prefix={0},local={0};
    if(!parse_name(s,&p,&name,&prefix,&local)) return false;
    while(!cancel_requested(s) && p<s->input.len && svg_xml_internal_whitespace(byte_at(s,p))) ++p;
    if(p>=s->input.len||byte_at(s,p)!='>') { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,p); return false; }
    if(s->depth==0||!qname_equal(s,name,s->stack[s->depth-1].span)) { fail(s,SVG_XML_ERROR_MISMATCHED_TAG,start); return false; }
    svg_xml_token t={0}; t.kind=SVG_XML_TOKEN_END_ELEMENT;t.name=name;t.prefix=prefix;t.local_name=local;t.depth=s->depth-1;
    (void)span_at(start,p+1,&t.source);
    if(!emit(s,t,start)) return false;
    --s->depth;s->cursor=p+1;return true;
}

static bool parse_start(xml_scanner *s)
{
    size_t start=s->cursor,p=start+1; svg_xml_span name={0},prefix={0},local={0};
    if(!parse_name(s,&p,&name,&prefix,&local)) return false;
    if(!valid_qname(s,prefix,local)) return false;
    if(s->depth==0) {
        if(s->root_count!=0) { fail(s,SVG_XML_ERROR_MULTIPLE_ROOTS,start); return false; }
        ++s->root_count;
    }
    if(s->depth>=s->options.limits.nesting_depth||s->depth>=XML_MAX_DEPTH) { fail(s,SVG_XML_ERROR_DEPTH_LIMIT,start); return false; }
    if(!count_node(s,start)) return false;
    svg_xml_token t={0};t.kind=SVG_XML_TOKEN_START_ELEMENT;t.name=name;t.prefix=prefix;t.local_name=local;t.depth=s->depth;
    (void)span_at(start,p,&t.source);
    size_t parent_index=s->result.token_count;
    if(!emit(s,t,start)) return false;
    size_t local_attrs=0;
    for(;;) {
        if (cancel_requested(s)) return false;
        bool separated=false;
        while(!cancel_requested(s) && p<s->input.len&&svg_xml_internal_whitespace(byte_at(s,p))) { ++p; separated=true; }
        if(p>=s->input.len) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,start);return false; }
        if(byte_at(s,p)=='>') {
            s->tokens[parent_index].source.length=p+1-start;
            s->stack[s->depth].span=name;++s->depth;s->cursor=p+1;
            return true;
        }
        if(byte_at(s,p)=='/'&&p+1<s->input.len&&byte_at(s,p+1)=='>') {
            if(parent_index>=s->result.token_count) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,start);return false; }
            s->tokens[parent_index].kind=SVG_XML_TOKEN_EMPTY_ELEMENT;
            s->tokens[parent_index].source.length=p+2-start;
            s->cursor=p+2;return true;
        }
        if(!separated) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,p);return false; }
        if(local_attrs>=s->options.limits.attributes_per_element) { fail(s,SVG_XML_ERROR_ATTRIBUTE_LIMIT,p);return false; }
        if(s->total_attributes>=s->options.limits.total_attributes) { fail(s,SVG_XML_ERROR_ATTRIBUTE_LIMIT,p);return false; }
        size_t attr_start=p; svg_xml_span an={0},ap={0},al={0};
        if(!parse_name(s,&p,&an,&ap,&al)) return false;
        if(!valid_qname(s,ap,al)) return false;
        for(size_t i=parent_index+1;i<s->result.token_count;++i) if(s->tokens[i].kind==SVG_XML_TOKEN_ATTRIBUTE && qname_equal(s,an,s->tokens[i].name)) {
            fail(s,SVG_XML_ERROR_DUPLICATE_ATTRIBUTE,attr_start);return false;
        }
        while(!cancel_requested(s) && p<s->input.len&&svg_xml_internal_whitespace(byte_at(s,p))) ++p;
        if(p>=s->input.len||byte_at(s,p)!='=') { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,p);return false; }
        ++p;while(!cancel_requested(s) && p<s->input.len&&svg_xml_internal_whitespace(byte_at(s,p))) ++p;
        if(p>=s->input.len||(byte_at(s,p)!='\''&&byte_at(s,p)!='"')) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,p);return false; }
        uint8_t quote=byte_at(s,p++);size_t value_start=p;
        while(!cancel_requested(s) && p<s->input.len&&byte_at(s,p)!=quote) {
            if(byte_at(s,p)=='<') { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,p);return false; }
            if(byte_at(s,p)=='&') { if(!scan_reference(s,&p)) return false; } else ++p;
        }
        if(p>=s->input.len) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,value_start-1);return false; }
        svg_xml_token a={0};a.kind=SVG_XML_TOKEN_ATTRIBUTE;a.name=an;a.prefix=ap;a.local_name=al;a.depth=s->depth;
        (void)span_at(attr_start,p+1,&a.source);(void)span_at(value_start,p,&a.value);
        if(!emit(s,a,attr_start)) return false;
        ++local_attrs;++s->total_attributes;++s->result.attribute_count;++p;
    }
}

static bool validate_decl_encoding(xml_scanner *s, const svg_xml_token *t)
{
    size_t p=t->value.offset, end=p+t->value.length;
    bool have_version=false,have_encoding=false,have_standalone=false;
    while(!cancel_requested(s) && p<end) {
        while(!cancel_requested(s) && p<end&&svg_xml_internal_whitespace(s->input.ptr[p])) ++p;
        if(p>=end) break;
        size_t ns=p;while(!cancel_requested(s) && p<end&&((s->input.ptr[p]>='A'&&s->input.ptr[p]<='Z')||(s->input.ptr[p]>='a'&&s->input.ptr[p]<='z')))++p;
        size_t nl=p-ns;while(!cancel_requested(s) && p<end&&svg_xml_internal_whitespace(s->input.ptr[p]))++p;
        if(p>=end||s->input.ptr[p++]!='=') { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,p);return false; }
        while(!cancel_requested(s) && p<end&&svg_xml_internal_whitespace(s->input.ptr[p]))++p;
        if(p>=end||(s->input.ptr[p]!='\''&&s->input.ptr[p]!='"')) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,p);return false; }
        uint8_t q=s->input.ptr[p++];size_t vs=p;while(!cancel_requested(s) && p<end&&s->input.ptr[p]!=q)++p;
        if(p>=end) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,vs-1);return false; }
        size_t vl=p-vs;++p;
        if(p<end&&!svg_xml_internal_whitespace(s->input.ptr[p])) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,p);return false; }
        if(nl==8&&memcmp(s->input.ptr+ns,"encoding",8)==0) {
            if(!have_version||have_encoding||have_standalone||vl==0 ||
               !((s->input.ptr[vs]>='A'&&s->input.ptr[vs]<='Z')||(s->input.ptr[vs]>='a'&&s->input.ptr[vs]<='z'))) {
                fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,ns);return false;
            }
            for(size_t i=0;i<vl;++i) {
                uint8_t c=s->input.ptr[vs+i];
                if(!((c>='A'&&c<='Z')||(c>='a'&&c<='z')||(c>='0'&&c<='9')||c=='.'||c=='_'||c=='-')) {
                    fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,vs+i);return false;
                }
            }
            bool utf8=vl==5;
            static const char expected[]="utf-8";
            for(size_t i=0;i<5&&utf8;++i) {
                uint8_t c=s->input.ptr[vs+i];
                if(c>='A'&&c<='Z') c=(uint8_t)(c-'A'+'a');
                if(c!=(uint8_t)expected[i]) utf8=false;
            }
            if(!utf8) { fail(s,SVG_XML_ERROR_UNSUPPORTED_ENCODING,vs);return false; }
            have_encoding=true;
        } else if(nl==7&&memcmp(s->input.ptr+ns,"version",7)==0) {
            if(have_version||have_encoding||have_standalone||vl!=3||memcmp(s->input.ptr+vs,"1.0",3)!=0) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,vs);return false; }
            have_version=true;
        } else if(nl==10&&memcmp(s->input.ptr+ns,"standalone",10)==0) {
            if(!have_version||have_standalone||!((vl==3&&memcmp(s->input.ptr+vs,"yes",3)==0)||
                                                 (vl==2&&memcmp(s->input.ptr+vs,"no",2)==0))) {
                fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,vs);return false;
            }
            have_standalone=true;
        } else {
            fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,ns);return false;
        }
    }
    if(!have_version) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,t->source.offset);return false; }
    return true;
}

static bool tokenize(xml_scanner *s)
{
    if(!base_bytes_valid(s->input)||s->tokens==NULL) { fail(s,SVG_XML_ERROR_INVALID_ARGUMENT,0);return false; }
    if(s->input.len>s->options.limits.input_bytes) { fail(s,SVG_XML_ERROR_INPUT_LIMIT,s->options.limits.input_bytes);return false; }
    if(!scan_chars(s)) return false;
    if (starts(s, 0, "\xef\xbb\xbf", 3)) s->cursor = 3;
    while(!cancel_requested(s) && s->cursor<s->input.len) {
        size_t p=s->cursor;
        if(byte_at(s,p)!='<') { if(!parse_text(s))return false; continue; }
        if(starts(s,p,"<!--",4)) { if(!parse_comment(s))return false; continue; }
        if(starts(s,p,"<?",2)) {
            size_t before=s->result.token_count;
            if(!parse_pi(s))return false;
            if(s->result.token_count>before&&s->tokens[before].kind==SVG_XML_TOKEN_DECLARATION&&
               !validate_decl_encoding(s,&s->tokens[before])) return false;
            continue;
        }
        if(starts(s,p,"<![CDATA[",9)) { if(!parse_cdata(s))return false;continue; }
        if(starts(s,p,"<!DOCTYPE",9)||starts(s,p,"<!ENTITY",8)) { fail(s,SVG_XML_ERROR_UNSUPPORTED_DTD,p);return false; }
        if(starts(s,p,"</",2)) { if(!parse_end(s))return false;continue; }
        if(starts(s,p,"<!",2)) { fail(s,SVG_XML_ERROR_MALFORMED_SYNTAX,p);return false; }
        if(!parse_start(s))return false;
        ++s->result.element_count;
    }
    if(s->depth!=0||s->root_count==0) { fail(s,SVG_XML_ERROR_INCOMPLETE_DOCUMENT,s->input.len);return false; }
    return true;
}

static bool stream_scan_characters(xml_scanner *s, size_t budget)
{
    size_t used = 0U;
    while (!cancel_requested(s) && used < budget && s->char_cursor < s->input.len) {
        uint8_t byte = byte_at(s, s->char_cursor);
        if (s->char_pending_length == 0U) {
            s->char_sequence_start = s->char_cursor;
            s->char_pending_width = byte < 0x80U ? 1U :
                (byte >= 0xc2U && byte <= 0xdfU) ? 2U :
                (byte >= 0xe0U && byte <= 0xefU) ? 3U :
                (byte >= 0xf0U && byte <= 0xf4U) ? 4U : 1U;
        }
        s->char_pending[s->char_pending_length++] = byte;
        ++s->char_cursor;
        ++used;
        if (s->char_pending_length == s->char_pending_width) {
            size_t next = 0U;
            uint32_t cp = 0U;
            if (!svg_xml_internal_utf8_next(s->char_pending, s->char_pending_length, &next, &cp) ||
                next != s->char_pending_length) {
                fail(s, SVG_XML_ERROR_INVALID_UTF8, s->char_sequence_start);
                return false;
            }
            if (!svg_xml_internal_char(cp)) {
                fail(s, SVG_XML_ERROR_INVALID_CHARACTER, s->char_sequence_start);
                return false;
            }
            s->char_pending_length = 0U;
            s->char_pending_width = 0U;
        }
    }
    if (s->char_cursor == s->input.len) {
        if (s->char_pending_length != 0U) {
            fail(s, SVG_XML_ERROR_INVALID_UTF8, s->char_sequence_start);
            return false;
        }
        s->chars_complete = true;
    }
    return !s->canceled && s->result.diagnostic.category == SVG_XML_ERROR_NONE;
}

#include "xml_stream_internal.inc"

static bool stream_parse_next(xml_scanner *s, size_t limit)
{
    size_t p = s->cursor;
    if (s->stream_start_active) return stream_parse_start(s, limit);
    if (s->stream_end_active) return stream_parse_end(s, limit);
    if (s->pending_kind == XML_STREAM_PENDING_TEXT || byte_at(s, p) != '<')
        return stream_parse_text(s, limit);
    if (s->pending_kind == XML_STREAM_PENDING_COMMENT || starts(s, p, "<!--", 4U))
        return stream_parse_comment(s, limit);
    if (s->pending_kind == XML_STREAM_PENDING_PI || starts(s, p, "<?", 2U))
        return stream_parse_pi(s, limit);
    if (s->pending_kind == XML_STREAM_PENDING_CDATA || starts(s, p, "<![CDATA[", 9U))
        return stream_parse_cdata(s, limit);
    if (starts(s, p, "<!DOCTYPE", 9U) || starts(s, p, "<!ENTITY", 8U)) {
        fail(s, SVG_XML_ERROR_UNSUPPORTED_DTD, p);
        return false;
    }
    if (starts(s, p, "</", 2U)) {
        return stream_parse_end(s, limit);
    }
    if (starts(s, p, "<!", 2U)) {
        fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, p);
        return false;
    }
    return stream_parse_start(s, limit);
}

static size_t stream_frontier(const xml_scanner *s)
{
    if (s->stream_start_active) return s->stream_start_pos;
    if (s->stream_end_active) return s->stream_end_pos;
    if (s->pending_kind == XML_STREAM_PENDING_NONE) return s->cursor;
    size_t frontier = s->pending_cursor;
    if (s->pending_data > frontier) frontier = s->pending_data;
    return frontier;
}

svg_xml_stream_status svg_xml_token_stream_start(base_bytes input, size_t max_storage_bytes,
    svg_xml_cancel_check_fn canceled, void *cancel_context, svg_xml_token_stream **out_stream)
{
    if (out_stream == NULL) return SVG_XML_STREAM_FAILED;
    *out_stream = NULL;
    if (!base_bytes_valid(input) || input.len > 1024U * 1024U * 1024U ||
        max_storage_bytes < sizeof(svg_xml_token)) return SVG_XML_STREAM_FAILED;
    svg_xml_token_stream *stream = calloc(1U, sizeof(*stream));
    if (stream == NULL) return SVG_XML_STREAM_FAILED;
    xml_scanner *s = &stream->scanner;
    s->input = input;
    s->options = svg_xml_default_options();
    s->options.limits.input_bytes = 1024U * 1024U * 1024U;
    s->result.diagnostic.category = SVG_XML_ERROR_NONE;
    s->tokens = NULL;
    s->capacity = 0U;
    s->token_storage_limit = max_storage_bytes;
    s->owns_tokens = true;
    s->cancel_check = canceled;
    s->cancel_context = cancel_context;
    if (starts(s, 0U, "\xef\xbb\xbf", 3U)) s->cursor = 3U;
    *out_stream = stream;
    return SVG_XML_STREAM_OK;
}

svg_xml_stream_status svg_xml_token_stream_step(svg_xml_token_stream *stream, size_t byte_budget)
{
    if (stream == NULL || byte_budget == 0U) return SVG_XML_STREAM_FAILED;
    xml_scanner *s = &stream->scanner;
    if (s->canceled) return SVG_XML_STREAM_CANCELED;
    if (s->result.diagnostic.category != SVG_XML_ERROR_NONE) return SVG_XML_STREAM_FAILED;
    if (s->stream_complete) return SVG_XML_STREAM_COMPLETE;
    if (byte_budget > 64U * 1024U) byte_budget = 64U * 1024U;
    size_t char_initial = s->char_cursor;
    if (!stream_scan_characters(s, byte_budget)) {
        if (s->canceled) return SVG_XML_STREAM_CANCELED;
        s->result.token_count = 0U;
        s->result.element_count = 0U;
        s->result.attribute_count = 0U;
        s->result.node_count = 0U;
        return SVG_XML_STREAM_FAILED;
    }
    if (!s->chars_complete) return SVG_XML_STREAM_OK;
    size_t char_work = s->char_cursor - char_initial;
    if (char_work >= byte_budget) return SVG_XML_STREAM_OK;
    size_t parse_budget = byte_budget - char_work;
    size_t initial = stream_frontier(s);
    size_t limit = 0U;
    if (!add_size(initial, parse_budget, &limit) || limit > s->input.len) limit = s->input.len;
    while (!cancel_requested(s) && s->cursor < s->input.len &&
           stream_frontier(s) - initial < parse_budget) {
        size_t before = stream_frontier(s);
        if (!stream_parse_next(s, limit) || stream_frontier(s) == before) break;
    }
    if (s->canceled) return SVG_XML_STREAM_CANCELED;
    if (s->result.diagnostic.category != SVG_XML_ERROR_NONE) {
        s->result.token_count = 0U;
        s->result.element_count = 0U;
        s->result.attribute_count = 0U;
        s->result.node_count = 0U;
        return SVG_XML_STREAM_FAILED;
    }
    if (s->chars_complete && s->cursor == s->input.len) {
        if (s->depth != 0U || s->root_count == 0U)
            fail(s, SVG_XML_ERROR_INCOMPLETE_DOCUMENT, s->input.len);
        else s->stream_complete = true;
        if (s->result.diagnostic.category != SVG_XML_ERROR_NONE) {
            s->result.token_count = 0U;
            s->result.element_count = 0U;
            s->result.attribute_count = 0U;
            s->result.node_count = 0U;
            return SVG_XML_STREAM_FAILED;
        }
    }
    return s->stream_complete ? SVG_XML_STREAM_COMPLETE : SVG_XML_STREAM_OK;
}

svg_xml_result svg_xml_token_stream_result(const svg_xml_token_stream *stream)
{
    svg_xml_result empty = {0};
    if (stream == NULL) {
        empty.diagnostic.category = SVG_XML_ERROR_INVALID_ARGUMENT;
        return empty;
    }
    return stream->scanner.result;
}

size_t svg_xml_token_stream_bytes_read(const svg_xml_token_stream *stream)
{
    return stream == NULL ? 0U : stream_frontier(&stream->scanner);
}

bool svg_xml_token_stream_take(svg_xml_token_stream *stream, svg_xml_token **out_tokens,
                               size_t *out_token_count)
{
    if (out_tokens == NULL || out_token_count == NULL) return false;
    *out_tokens = NULL;
    *out_token_count = 0U;
    if (stream == NULL || !stream->scanner.stream_complete) return false;
    *out_tokens = stream->scanner.tokens;
    *out_token_count = stream->scanner.result.token_count;
    stream->scanner.tokens = NULL;
    stream->scanner.capacity = 0U;
    stream->scanner.owns_tokens = false;
    return true;
}

void svg_xml_token_stream_destroy(svg_xml_token_stream *stream)
{
    if (stream == NULL) return;
    if (stream->scanner.owns_tokens) free(stream->scanner.tokens);
    free(stream);
}

svg_xml_result svg_xml_tokenizer_run(base_bytes input, svg_xml_options options,
    svg_xml_token *tokens, size_t token_capacity, svg_xml_cancel_check_fn canceled,
    void *cancel_context)
{
    xml_scanner s = {0};
    s.input = input; s.options = options; s.tokens = tokens; s.capacity = token_capacity;
    s.cancel_check = canceled; s.cancel_context = cancel_context;
    s.result.diagnostic.category = SVG_XML_ERROR_NONE;
    if (tokenize(&s)) return s.result;
    s.result.token_count = 0U;
    s.result.element_count = 0U; s.result.attribute_count = 0U; s.result.node_count = 0U;
    return s.result;
}
