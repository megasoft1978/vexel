#include "xml_tokenizer.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>

static double now_seconds(void)
{
    struct timeval tv;
    if(gettimeofday(&tv,NULL)!=0)return 0.0;
    return (double)tv.tv_sec+(double)tv.tv_usec/1000000.0;
}

static uint32_t next_random(uint32_t *state)
{
    *state^=*state<<13U;*state^=*state>>17U;*state^=*state<<5U;return *state;
}

static int invariant(base_bytes in, const svg_xml_result *r, const svg_xml_token *tokens)
{
    if(r->diagnostic.offset>in.len)return 0;
    if(r->diagnostic.category!=SVG_XML_ERROR_NONE)return r->token_count==0;
    for(size_t i=0;i<r->token_count;++i) {
        const svg_xml_span spans[]={tokens[i].source,tokens[i].name,tokens[i].value,tokens[i].prefix,tokens[i].local_name};
        for(size_t j=0;j<sizeof(spans)/sizeof(spans[0]);++j)
            if(spans[j].offset>in.len||spans[j].length>in.len-spans[j].offset)return 0;
    }
    return 1;
}

int main(int argc,char **argv)
{
    if(argc!=3)return 2;
    char *end=NULL;unsigned long seed=strtoul(argv[1],&end,10);if(end==argv[1]||*end!='\0'||seed>UINT32_MAX)return 2;
    end=NULL;unsigned long duration=strtoul(argv[2],&end,10);if(end==argv[2]||*end!='\0'||duration==0||duration>3600)return 2;
    uint32_t state=(uint32_t)seed;uint8_t bytes[2048];svg_xml_token tokens[2048];
    static const char *const motifs[]={"<a/>","<svg xmlns='urn:x'><g></svg>","<!--x-->","<![CDATA[x]]>","<?p x?>","<a x='&amp;'/>","<!DOCTYPE a><a/>","<a>]]></a>","<a x=\"x\" x='y'/>","</a>"};
    uint64_t iterations=0;double start=now_seconds(),deadline=start+(double)duration;
    while(now_seconds()<deadline) {
        size_t n=(size_t)(next_random(&state)%sizeof(bytes));
        for(size_t i=0;i<n;++i)bytes[i]=(uint8_t)next_random(&state);
        if((iterations%3U)==0) {
            const char *m=motifs[next_random(&state)%(sizeof(motifs)/sizeof(motifs[0]))];
            size_t mlen=strlen(m);if(mlen>sizeof(bytes))return 3;memcpy(bytes,m,mlen);n=mlen;
        }
        base_bytes in={bytes,n};svg_xml_result r=svg_xml_tokenize(in,svg_xml_default_options(),tokens,sizeof(tokens)/sizeof(tokens[0]));
        if(!invariant(in,&r,tokens)) { fprintf(stderr,"span/offset invariant failed at iteration %llu\n",(unsigned long long)iterations);return 4; }
        ++iterations;
    }
    double elapsed=now_seconds()-start;
    printf("{\"seed\":%lu,\"iterations\":%llu,\"elapsed_seconds\":%.6f,\"sanitizer_status\":\"clean\"}\n",seed,(unsigned long long)iterations,elapsed);
    return elapsed+0.005<(double)duration?5:0;
}
