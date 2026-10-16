#include "xml_tokenizer.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { \
    fprintf(stderr, "check failed at %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    abort(); \
} } while (0)

static svg_xml_result run(const char *text, svg_xml_options options, svg_xml_token *tokens, size_t cap)
{
    base_bytes input={(const uint8_t *)text,strlen(text)};
    svg_xml_result r=svg_xml_tokenize(input,options,tokens,cap);
    CHECK(r.diagnostic.offset<=input.len);
    if(r.diagnostic.category==SVG_XML_ERROR_NONE) {
        for(size_t i=0;i<r.token_count;++i) {
            CHECK(tokens[i].source.offset<=input.len);
            CHECK(tokens[i].source.length<=input.len-tokens[i].source.offset);
            CHECK(tokens[i].name.offset<=input.len);
            CHECK(tokens[i].name.length<=input.len-tokens[i].name.offset);
            CHECK(tokens[i].value.offset<=input.len);
            CHECK(tokens[i].value.length<=input.len-tokens[i].value.offset);
            CHECK(tokens[i].prefix.offset<=input.len);
            CHECK(tokens[i].prefix.length<=input.len-tokens[i].prefix.offset);
            CHECK(tokens[i].local_name.offset<=input.len);
            CHECK(tokens[i].local_name.length<=input.len-tokens[i].local_name.offset);
        }
    } else CHECK(r.token_count==0);
    return r;
}

static void expect_error(const char *s, svg_xml_error e)
{
    svg_xml_token tokens[64];svg_xml_result r=run(s,svg_xml_default_options(),tokens,64);
    CHECK(r.diagnostic.category==e);
}

static uint32_t rng_next(uint32_t *state)
{
    *state^=*state<<13U;*state^=*state>>17U;*state^=*state<<5U;return *state;
}

static bool span_is(const char *source, svg_xml_span span, const char *expected)
{
    size_t n=strlen(expected);
    return span.length==n&&memcmp(source+span.offset,expected,n)==0;
}

static void check_doctype(void)
{
    svg_xml_token tokens[64];
    static const char *const accepted[] = {
        "<!DOCTYPE a><a/>",
        "<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\"\n \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\"><a/>",
        "<!DOCTYPE svg [\n<!ENTITY ns \"http://www.w3.org/2000/svg\">\n]><a/>",
        "<!DOCTYPE svg [<!-- ]> \">\" --><!ENTITY e \"a>b]\"><?pi ]> ?>]><a/>",
        "<?xml version='1.0'?>\n<!DOCTYPE svg [<!ELEMENT a ANY>]>\n<a/>"};
    for(size_t i=0;i<sizeof(accepted)/sizeof(accepted[0]);++i) {
        svg_xml_result r=run(accepted[i],svg_xml_default_options(),tokens,64);
        CHECK(r.diagnostic.category==SVG_XML_ERROR_NONE);
        size_t found=SIZE_MAX;
        for(size_t k=0;k<r.token_count;++k)if(tokens[k].kind==SVG_XML_TOKEN_DOCTYPE)found=k;
        CHECK(found!=SIZE_MAX);
        const char *end=strstr(accepted[i],"<a/>");
        CHECK(end!=NULL);
        size_t start=(size_t)(strstr(accepted[i],"<!DOCTYPE")-accepted[i]);
        size_t stop=(size_t)(end-accepted[i]);
        while(accepted[i][stop-1]=='\n')--stop;
        CHECK(tokens[found].source.offset==start&&tokens[found].source.length==stop-start);
        CHECK(r.element_count==1);
    }
    /* A declared entity is skipped, never expanded: its reference still fails. */
    expect_error("<!DOCTYPE a [<!ENTITY e \"x\">]><a>&e;</a>",SVG_XML_ERROR_MALFORMED_SYNTAX);
    expect_error("<!DOCTYPE a [<!ENTITY e \"x\">]><a v='&e;'/>",SVG_XML_ERROR_MALFORMED_SYNTAX);
    expect_error("<!DOCTYPE a><a/><!DOCTYPE b>",SVG_XML_ERROR_MALFORMED_SYNTAX);
    expect_error("<a/><!DOCTYPE a>",SVG_XML_ERROR_MALFORMED_SYNTAX);
    expect_error("<a><!DOCTYPE a></a>",SVG_XML_ERROR_MALFORMED_SYNTAX);
    expect_error("<!DOCTYPE a><!DOCTYPE a><a/>",SVG_XML_ERROR_MALFORMED_SYNTAX);
    expect_error("<!DOCTYPEa><a/>",SVG_XML_ERROR_MALFORMED_SYNTAX);
    expect_error("<!DOCTYPE a [<!ENTITY e \"x\"><a/>",SVG_XML_ERROR_MALFORMED_SYNTAX);
    expect_error("<!DOCTYPE a",SVG_XML_ERROR_MALFORMED_SYNTAX);
    expect_error("<!ENTITY e \"x\"><a/>",SVG_XML_ERROR_UNSUPPORTED_DTD);
}

static void check_attribute_decoder(void)
{
    static const uint8_t source[] = "a&amp;&#65;&#x1f600;z";
    uint8_t output[32] = {0};
    size_t length = 0U, error = SIZE_MAX;
    CHECK(svg_xml_decode_attribute_value((base_bytes){source, sizeof(source) - 1U},
        (svg_xml_span){0U, sizeof(source) - 1U}, output, sizeof(output),
        &length, &error));
    static const uint8_t expected[] = "a&A\xf0\x9f\x98\x80z";
    CHECK(length == sizeof(expected) - 1U &&
          memcmp(output, expected, sizeof(expected) - 1U) == 0);
    static const uint8_t named[] = "&lt;&gt;&quot;&apos;&amp;";
    CHECK(svg_xml_decode_attribute_value((base_bytes){named, sizeof(named) - 1U},
        (svg_xml_span){0U, sizeof(named) - 1U}, output, sizeof(output),
        &length, &error));
    static const uint8_t named_expected[] = "<>\"'&";
    CHECK(length == sizeof(named_expected) - 1U &&
          memcmp(output, named_expected, sizeof(named_expected) - 1U) == 0);
    static const char *const invalid[] = {
        "&unknown;", "&#;", "&#x;", "&#1114112;", "&#x110000;",
        "&#xD800;", "&#0;", "&amp", "&#999999999999999999;"
    };
    for (size_t i = 0U; i < sizeof(invalid) / sizeof(invalid[0]); ++i) {
        base_bytes input = {(const uint8_t *)invalid[i], strlen(invalid[i])};
        CHECK(!svg_xml_decode_attribute_value(input,
            (svg_xml_span){0U, input.len}, output, sizeof(output),
            &length, &error));
        CHECK(error == 0U);
    }
    static const uint8_t too_small[] = "&#x1f600;";
    CHECK(!svg_xml_decode_attribute_value((base_bytes){too_small,
        sizeof(too_small) - 1U}, (svg_xml_span){0U, sizeof(too_small) - 1U},
        output, 3U, &length, &error));
    CHECK(error == 0U);
    static const uint8_t prefixed[] = "xx&bad;";
    CHECK(!svg_xml_decode_attribute_value((base_bytes){prefixed,
        sizeof(prefixed) - 1U}, (svg_xml_span){2U, sizeof(prefixed) - 3U},
        output, sizeof(output), &length, &error));
    CHECK(error == 0U);
}

int main(void)
{
    check_attribute_decoder();
    const char source[]="<?xml version='1.0' encoding=\"UTF-8\"?>\r\n<!--c--><?p data?><a:root xmlns:a='urn:x' a:k=\"v&amp;x\">x\r\ny<![CDATA[z]]><a:child/></a:root>";
    char original[sizeof(source)];memcpy(original,source,sizeof(source));
    svg_xml_token tokens[64];svg_xml_result r=run(source,svg_xml_default_options(),tokens,64);
    CHECK(r.diagnostic.category==SVG_XML_ERROR_NONE&&r.element_count==2&&r.attribute_count==2);
    CHECK(memcmp(source,original,sizeof(source))==0);
    bool declaration=false,comment=false,pi=false,start=false,end=false,empty=false,text=false,cdata=false,qname=false;
    bool attr_exact=false,comment_exact=false,pi_exact=false,text_exact=false,cdata_exact=false;
    bool qname_exact=false,start_source_exact=false;
    for(size_t i=0;i<r.token_count;++i) {
        svg_xml_token *t=&tokens[i];
        declaration|=t->kind==SVG_XML_TOKEN_DECLARATION;comment|=t->kind==SVG_XML_TOKEN_COMMENT;
        pi|=t->kind==SVG_XML_TOKEN_PROCESSING_INSTRUCTION;start|=t->kind==SVG_XML_TOKEN_START_ELEMENT;
        end|=t->kind==SVG_XML_TOKEN_END_ELEMENT;empty|=t->kind==SVG_XML_TOKEN_EMPTY_ELEMENT;
        text|=t->kind==SVG_XML_TOKEN_TEXT;cdata|=t->kind==SVG_XML_TOKEN_CDATA;
        if(t->kind==SVG_XML_TOKEN_START_ELEMENT&&t->prefix.length==1&&t->local_name.length==4)qname=true;
        if(t->kind==SVG_XML_TOKEN_START_ELEMENT&&span_is(source,t->prefix,"a")&&
           span_is(source,t->local_name,"root"))qname_exact=true;
        if(t->kind==SVG_XML_TOKEN_START_ELEMENT&&
           span_is(source,t->source,"<a:root xmlns:a='urn:x' a:k=\"v&amp;x\">"))
            start_source_exact=true;
        if(t->kind==SVG_XML_TOKEN_ATTRIBUTE&&span_is(source,t->name,"a:k")&&span_is(source,t->value,"v&amp;x"))attr_exact=true;
        if(t->kind==SVG_XML_TOKEN_COMMENT&&span_is(source,t->value,"c"))comment_exact=true;
        if(t->kind==SVG_XML_TOKEN_PROCESSING_INSTRUCTION&&span_is(source,t->value,"data"))pi_exact=true;
        if(t->kind==SVG_XML_TOKEN_TEXT&&span_is(source,t->value,"x\r\ny"))text_exact=true;
        if(t->kind==SVG_XML_TOKEN_CDATA&&span_is(source,t->value,"z"))cdata_exact=true;
    }
    CHECK(declaration&&comment&&pi&&start&&end&&empty&&text&&cdata&&qname);
    CHECK(qname_exact&&start_source_exact);
    CHECK(attr_exact&&comment_exact&&pi_exact&&text_exact&&cdata_exact);
    expect_error("<a x='1' x=\"2\"/>",SVG_XML_ERROR_DUPLICATE_ATTRIBUTE);
    expect_error("<a x='1'y='2'/>",SVG_XML_ERROR_MALFORMED_SYNTAX);
    expect_error("<a:1/>",SVG_XML_ERROR_INVALID_NAME);
    expect_error("<a><b></a>",SVG_XML_ERROR_MISMATCHED_TAG);
    expect_error("<a/><b/>",SVG_XML_ERROR_MULTIPLE_ROOTS);
    expect_error("<a>",SVG_XML_ERROR_INCOMPLETE_DOCUMENT);
    check_doctype();
    expect_error("<a><!-- -- --></a>",SVG_XML_ERROR_MALFORMED_SYNTAX);
    expect_error("<a><?x nope</a>",SVG_XML_ERROR_MALFORMED_SYNTAX);
    expect_error("<a><![CDATA[x</a>",SVG_XML_ERROR_MALFORMED_SYNTAX);
    expect_error("<a x='unterminated/>",SVG_XML_ERROR_MALFORMED_SYNTAX);
    expect_error("<a>&unknown;</a>",SVG_XML_ERROR_MALFORMED_SYNTAX);
    expect_error("<?xml version='1.0' encoding='UTF-16'?><a/>",SVG_XML_ERROR_UNSUPPORTED_ENCODING);
    expect_error("<?xml version='1.0'encoding='UTF-8'?><a/>",SVG_XML_ERROR_MALFORMED_SYNTAX);
    expect_error("<a>\001</a>",SVG_XML_ERROR_INVALID_CHARACTER);
    expect_error("<1a/>",SVG_XML_ERROR_INVALID_NAME);
    expect_error("<:a/>",SVG_XML_ERROR_INVALID_NAME);
    expect_error("<a>&#0;</a>",SVG_XML_ERROR_INVALID_CHARACTER);
    r=run("<a><?empty?></a>",svg_xml_default_options(),tokens,64);CHECK(r.diagnostic.category==SVG_XML_ERROR_NONE);
    r=run("<?xml version='1.0' standalone='yes'?><a/>",svg_xml_default_options(),tokens,64);CHECK(r.diagnostic.category==SVG_XML_ERROR_NONE);
    r=run("<?a:b:c data?><a/>",svg_xml_default_options(),tokens,64);
    CHECK(r.diagnostic.category==SVG_XML_ERROR_NONE&&span_is("<?a:b:c data?><a/>",tokens[0].name,"a:b:c"));
    r=run("<?a:1 data?><a/>",svg_xml_default_options(),tokens,64);
    CHECK(r.diagnostic.category==SVG_XML_ERROR_NONE&&span_is("<?a:1 data?><a/>",tokens[0].name,"a:1"));
    const char bom_document[]="\xef\xbb\xbf<?xml version='1.0'?><a/>";
    r=run(bom_document,svg_xml_default_options(),tokens,64);
    CHECK(r.diagnostic.category==SVG_XML_ERROR_NONE);
    CHECK(tokens[0].kind==SVG_XML_TOKEN_DECLARATION&&tokens[0].source.offset==3);

    svg_xml_options o=svg_xml_default_options();o.limits.nesting_depth=1;
    CHECK(run("<a><b/></a>",o,tokens,64).diagnostic.category==SVG_XML_ERROR_DEPTH_LIMIT);
    o=svg_xml_default_options();o.limits.nodes=1;
    CHECK(run("<a>x</a>",o,tokens,64).diagnostic.category==SVG_XML_ERROR_NODE_LIMIT);
    o=svg_xml_default_options();o.limits.attributes_per_element=1;
    CHECK(run("<a x='1' y='2'/>",o,tokens,64).diagnostic.category==SVG_XML_ERROR_ATTRIBUTE_LIMIT);
    o=svg_xml_default_options();o.limits.total_attributes=1;
    CHECK(run("<a x='1'><b y='2'/></a>",o,tokens,64).diagnostic.category==SVG_XML_ERROR_ATTRIBUTE_LIMIT);
    o=svg_xml_default_options();o.limits.text_run_bytes=2;
    CHECK(run("<a>abc</a>",o,tokens,64).diagnostic.category==SVG_XML_ERROR_TEXT_LIMIT);
    CHECK(run("<a><![CDATA[abc]]></a>",o,tokens,64).diagnostic.category==SVG_XML_ERROR_TEXT_LIMIT);
    o=svg_xml_default_options();o.limits.input_bytes=2;
    CHECK(run("<a/>",o,tokens,64).diagnostic.category==SVG_XML_ERROR_INPUT_LIMIT);
    o=svg_xml_default_options();o.limits.input_bytes=SIZE_MAX;
    size_t oversized_length=(16U*1024U*1024U)+1U;
    uint8_t *oversized=(uint8_t *)malloc(oversized_length);CHECK(oversized!=NULL);
    base_bytes oversized_input={oversized,oversized_length};
    r=svg_xml_tokenize(oversized_input,o,tokens,64);
    CHECK(r.diagnostic.category==SVG_XML_ERROR_INPUT_LIMIT&&r.diagnostic.offset==16U*1024U*1024U);
    free(oversized);
    CHECK(run("<a/>",svg_xml_default_options(),tokens,0).diagnostic.category==SVG_XML_ERROR_OUTPUT_EXHAUSTED);
    CHECK(run("<a x='1'/>",svg_xml_default_options(),tokens,1).diagnostic.category==SVG_XML_ERROR_OUTPUT_EXHAUSTED);

    const uint8_t bad_utf8[]={ '<','a','>',0xc0,0xaf,'<','/','a','>' };
    base_bytes in={bad_utf8,sizeof(bad_utf8)};r=svg_xml_tokenize(in,svg_xml_default_options(),tokens,64);
    CHECK(r.diagnostic.category==SVG_XML_ERROR_INVALID_UTF8&&r.diagnostic.offset<=in.len);
    const char *alphabet="<>/!?-=[]'\" abcxyz:012;&#\r\n";
    uint32_t seed=20261009U;size_t generated=0;
    uint8_t fuzz[96];
    for(size_t n=0;n<100000;++n) {
        size_t len=(size_t)(rng_next(&seed)%sizeof(fuzz));
        for(size_t i=0;i<len;++i)fuzz[i]=(uint8_t)alphabet[rng_next(&seed)%(sizeof("<>/!?-=[]'\" abcxyz:012;&#\r\n")-1U)];
        in.ptr=fuzz;in.len=len;r=svg_xml_tokenize(in,svg_xml_default_options(),tokens,64);
        CHECK(r.diagnostic.offset<=len);
        if(r.diagnostic.category==SVG_XML_ERROR_NONE)for(size_t i=0;i<r.token_count;++i) {
            CHECK(tokens[i].source.offset<=len&&tokens[i].source.length<=len-tokens[i].source.offset);
            CHECK(tokens[i].name.offset<=len&&tokens[i].name.length<=len-tokens[i].name.offset);
            CHECK(tokens[i].value.offset<=len&&tokens[i].value.length<=len-tokens[i].value.offset);
            CHECK(tokens[i].prefix.offset<=len&&tokens[i].prefix.length<=len-tokens[i].prefix.offset);
            CHECK(tokens[i].local_name.offset<=len&&tokens[i].local_name.length<=len-tokens[i].local_name.offset);
        }
        ++generated;
    }
    printf("xml tokenizer focused C checks passed; deterministic malformed inputs=%zu\n",generated);
    return 0;
}
