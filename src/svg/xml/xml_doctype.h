#ifndef VEXEL_XML_DOCTYPE_H
#define VEXEL_XML_DOCTYPE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* Byte-at-a-time recognizer that skips a <!DOCTYPE ...> declaration, including
 * an internal subset, without interpreting it. Entities are never declared or
 * expanded: the subset is only scanned for the bytes that can end it (quotes,
 * comments, processing instructions and the closing bracket). The state is
 * small and resumable so both the one-shot and the budgeted stream tokenizers
 * and the tag reader share one definition. */
typedef enum xml_doctype_phase {
    XML_DOCTYPE_START = 0,   /* just after "<!DOCTYPE": whitespace required */
    XML_DOCTYPE_MAIN,        /* outside any subset or literal */
    XML_DOCTYPE_QUOTE,       /* inside a public/system literal */
    XML_DOCTYPE_SUBSET,      /* inside [ ... ] */
    XML_DOCTYPE_SUBSET_QUOTE,
    XML_DOCTYPE_LT,          /* subset: saw '<' */
    XML_DOCTYPE_BANG,        /* subset: saw "<!" */
    XML_DOCTYPE_BANG_DASH,   /* subset: saw "<!-" */
    XML_DOCTYPE_COMMENT,
    XML_DOCTYPE_PI
} xml_doctype_phase;

typedef enum xml_doctype_result {
    XML_DOCTYPE_MORE = 0,
    XML_DOCTYPE_DONE,
    XML_DOCTYPE_BAD
} xml_doctype_result;

typedef struct xml_doctype_state {
    xml_doctype_phase phase;
    uint8_t quote;
    uint8_t run;            /* trailing '-' count in a comment, '?' flag in a PI */
    bool subset_closed;
} xml_doctype_state;

static inline xml_doctype_result xml_doctype_step(xml_doctype_state *d, uint8_t c)
{
    for (;;) {
        switch (d->phase) {
        case XML_DOCTYPE_START:
            if (c != ' ' && c != '\t' && c != '\r' && c != '\n') return XML_DOCTYPE_BAD;
            d->phase = XML_DOCTYPE_MAIN;
            return XML_DOCTYPE_MORE;
        case XML_DOCTYPE_MAIN:
            if (c == '"' || c == '\'') { d->quote = c; d->phase = XML_DOCTYPE_QUOTE; }
            else if (c == '[') {
                if (d->subset_closed) return XML_DOCTYPE_BAD;
                d->phase = XML_DOCTYPE_SUBSET;
            } else if (c == '>') return XML_DOCTYPE_DONE;
            else if (c == '<') return XML_DOCTYPE_BAD;
            return XML_DOCTYPE_MORE;
        case XML_DOCTYPE_QUOTE:
            if (c == d->quote) d->phase = XML_DOCTYPE_MAIN;
            return XML_DOCTYPE_MORE;
        case XML_DOCTYPE_SUBSET:
            if (c == '"' || c == '\'') { d->quote = c; d->phase = XML_DOCTYPE_SUBSET_QUOTE; }
            else if (c == '<') d->phase = XML_DOCTYPE_LT;
            else if (c == ']') { d->subset_closed = true; d->phase = XML_DOCTYPE_MAIN; }
            return XML_DOCTYPE_MORE;
        case XML_DOCTYPE_SUBSET_QUOTE:
            if (c == d->quote) d->phase = XML_DOCTYPE_SUBSET;
            return XML_DOCTYPE_MORE;
        case XML_DOCTYPE_LT:
            if (c == '!') { d->phase = XML_DOCTYPE_BANG; return XML_DOCTYPE_MORE; }
            if (c == '?') { d->run = 0U; d->phase = XML_DOCTYPE_PI; return XML_DOCTYPE_MORE; }
            d->phase = XML_DOCTYPE_SUBSET;
            continue;
        case XML_DOCTYPE_BANG:
            if (c == '-') { d->phase = XML_DOCTYPE_BANG_DASH; return XML_DOCTYPE_MORE; }
            d->phase = XML_DOCTYPE_SUBSET;
            continue;
        case XML_DOCTYPE_BANG_DASH:
            if (c == '-') { d->run = 0U; d->phase = XML_DOCTYPE_COMMENT; return XML_DOCTYPE_MORE; }
            d->phase = XML_DOCTYPE_SUBSET;
            continue;
        case XML_DOCTYPE_COMMENT:
            if (c == '-') { if (d->run < 2U) ++d->run; }
            else if (c == '>' && d->run >= 2U) d->phase = XML_DOCTYPE_SUBSET;
            else d->run = 0U;
            return XML_DOCTYPE_MORE;
        case XML_DOCTYPE_PI:
            if (c == '>' && d->run == 1U) d->phase = XML_DOCTYPE_SUBSET;
            else d->run = c == '?' ? 1U : 0U;
            return XML_DOCTYPE_MORE;
        }
        return XML_DOCTYPE_BAD;
    }
}

#endif
