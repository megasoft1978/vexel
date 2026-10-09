#include "xml_tokenizer.h"

#include <stdio.h>
#include <stdlib.h>

int main(int argc,char **argv)
{
    if(argc!=2)return 2;
    FILE *f=fopen(argv[1],"rb");if(f==NULL)return 2;
    if(fseek(f,0,SEEK_END)!=0){fclose(f);return 2;}
    long size=ftell(f);if(size<0||size>16L*1024L*1024L){fclose(f);return 2;}
    rewind(f);size_t n=(size_t)size;uint8_t *bytes=malloc(n?n:1);if(bytes==NULL){fclose(f);return 2;}
    if(fread(bytes,1,n,f)!=n){free(bytes);fclose(f);return 2;}fclose(f);
    svg_xml_token *tokens=calloc(n/2U+1U,sizeof(*tokens));if(tokens==NULL){free(bytes);return 2;}
    base_bytes input={bytes,n};svg_xml_result r=svg_xml_tokenize(input,svg_xml_default_options(),tokens,n/2U+1U);
    printf("{\"ok\":%s,\"error\":\"%s\",\"offset\":%zu,\"elements\":%zu,\"attributes\":%zu}\n",
        r.diagnostic.category==SVG_XML_ERROR_NONE?"true":"false",svg_xml_error_name(r.diagnostic.category),r.diagnostic.offset,r.element_count,r.attribute_count);
    free(tokens);free(bytes);return 0;
}
