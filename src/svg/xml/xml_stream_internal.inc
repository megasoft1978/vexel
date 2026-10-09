/* Included once by xml_tokenizer.c after shared scanner helpers. */
static void stream_pending_start(xml_scanner *s, xml_stream_pending_kind kind,
                                 size_t start, size_t cursor)
{
    s->pending_kind = kind;
    s->pending_start = start;
    s->pending_cursor = cursor;
    s->pending_data = 0U;
    s->pending_first_nonspace = 0U;
    s->pending_name = (svg_xml_span){0};
    s->pending_quote = 0U;
    s->pending_nonspace = false;
    s->pending_declaration = false;
    s->pending_pi_data_ready = false;
}

static void stream_pending_clear(xml_scanner *s)
{
    s->pending_kind = XML_STREAM_PENDING_NONE;
}

static bool stream_emit_text(xml_scanner *s, size_t end)
{
    size_t start = s->cursor;
    if (end == start) return true;
    if (end - start > s->options.limits.text_run_bytes) {
        fail(s, SVG_XML_ERROR_TEXT_LIMIT, start);
        return false;
    }
    if (s->depth == 0U && s->pending_nonspace) {
        fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, s->pending_first_nonspace);
        return false;
    }
    svg_xml_token token = {0};
    token.kind = SVG_XML_TOKEN_TEXT;
    token.depth = s->depth;
    (void)span_at(start, end, &token.source);
    token.value = token.source;
    if ((s->depth != 0U || s->pending_nonspace) && !count_node(s, start)) return false;
    if (!emit(s, token, start)) return false;
    s->cursor = end;
    stream_pending_clear(s);
    return true;
}

static bool stream_parse_text(xml_scanner *s, size_t limit)
{
    if (s->pending_kind != XML_STREAM_PENDING_TEXT || s->pending_start != s->cursor)
        stream_pending_start(s, XML_STREAM_PENDING_TEXT, s->cursor, s->cursor);
    size_t p = s->pending_cursor;
    while (p < limit && p < s->input.len && byte_at(s, p) != '<') {
        if (p + 3U > limit && limit < s->input.len) break;
        if (p + 3U <= s->input.len && starts(s, p, "]]>", 3U)) {
            fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, p);
            return false;
        }
        if (byte_at(s, p) == '&') {
            size_t semi = p;
            while (semi < limit && semi < s->input.len && byte_at(s, semi) != ';') ++semi;
            if (semi >= limit && semi < s->input.len) {
                s->pending_cursor = p;
                return true;
            }
            if (!scan_reference(s, &p)) return false;
        } else ++p;
        if (!s->pending_nonspace && !svg_xml_internal_whitespace(byte_at(s, p - 1U))) {
            s->pending_nonspace = true;
            s->pending_first_nonspace = p - 1U;
        }
    }
    s->pending_cursor = p;
    if (p < s->input.len && byte_at(s, p) == '<') return stream_emit_text(s, p);
    if (p == s->input.len) return stream_emit_text(s, p);
    return true;
}

static bool stream_parse_comment(xml_scanner *s, size_t limit)
{
    if (s->pending_kind != XML_STREAM_PENDING_COMMENT || s->pending_start != s->cursor)
        stream_pending_start(s, XML_STREAM_PENDING_COMMENT, s->cursor, s->cursor + 4U);
    size_t p = s->pending_cursor;
    while (p < limit && p < s->input.len) {
        if (p + 3U > limit && limit < s->input.len) break;
        if (p + 3U <= limit && starts(s, p, "-->", 3U)) {
            svg_xml_token token = {0};
            token.kind = SVG_XML_TOKEN_COMMENT;
            token.depth = s->depth;
            (void)span_at(s->cursor, p + 3U, &token.source);
            (void)span_at(s->cursor + 4U, p, &token.value);
            if (!count_node(s, s->cursor) || !emit(s, token, s->cursor)) return false;
            s->cursor = p + 3U;
            stream_pending_clear(s);
            return true;
        }
        if (p + 2U <= limit && starts(s, p, "--", 2U)) {
            fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, p);
            return false;
        }
        ++p;
    }
    s->pending_cursor = p;
    if (p == s->input.len) fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, s->cursor);
    return s->result.diagnostic.category == SVG_XML_ERROR_NONE;
}

static bool stream_parse_cdata(xml_scanner *s, size_t limit)
{
    if (s->pending_kind != XML_STREAM_PENDING_CDATA || s->pending_start != s->cursor)
        stream_pending_start(s, XML_STREAM_PENDING_CDATA, s->cursor, s->cursor + 9U);
    size_t p = s->pending_cursor;
    while (p < limit && p < s->input.len) {
        if (p + 3U > limit && limit < s->input.len) break;
        if (p + 3U <= limit && starts(s, p, "]]>", 3U)) {
            if (s->depth == 0U) { fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, s->cursor); return false; }
            if (p - (s->cursor + 9U) > s->options.limits.text_run_bytes) {
                fail(s, SVG_XML_ERROR_TEXT_LIMIT, s->cursor + 9U);
                return false;
            }
            svg_xml_token token = {0};
            token.kind = SVG_XML_TOKEN_CDATA;
            token.depth = s->depth;
            (void)span_at(s->cursor, p + 3U, &token.source);
            (void)span_at(s->cursor + 9U, p, &token.value);
            if (!count_node(s, s->cursor) || !emit(s, token, s->cursor)) return false;
            s->cursor = p + 3U;
            stream_pending_clear(s);
            return true;
        }
        ++p;
    }
    s->pending_cursor = p;
    if (p == s->input.len) fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, s->cursor);
    return s->result.diagnostic.category == SVG_XML_ERROR_NONE;
}

static bool stream_parse_pi(xml_scanner *s, size_t limit)
{
    if (s->pending_kind != XML_STREAM_PENDING_PI || s->pending_start != s->cursor) {
        size_t p = s->cursor + 2U;
        svg_xml_span name = {0};
        if (!parse_pi_target(s, &p, &name)) return false;
        bool declaration = name.length == 3U &&
            memcmp(s->input.ptr + name.offset, "xml", 3U) == 0;
        bool has_bom = starts(s, 0U, "\xef\xbb\xbf", 3U);
        if (declaration && s->cursor != 0U && !(has_bom && s->cursor == 3U)) {
            fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, s->cursor);
            return false;
        }
        if (!declaration && name.length == 3U) {
            uint8_t a = s->input.ptr[name.offset];
            uint8_t b = s->input.ptr[name.offset + 1U];
            uint8_t c = s->input.ptr[name.offset + 2U];
            if ((a == 'x' || a == 'X') && (b == 'm' || b == 'M') &&
                (c == 'l' || c == 'L')) {
                fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, s->cursor);
                return false;
            }
        }
        stream_pending_start(s, XML_STREAM_PENDING_PI, s->cursor, p);
        s->pending_name = name;
        s->pending_declaration = declaration;
    }
    size_t p = s->pending_cursor;
    if (!s->pending_pi_data_ready) {
        if (p >= limit) {
            if (p == s->input.len) fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, s->cursor);
            return s->result.diagnostic.category == SVG_XML_ERROR_NONE;
        }
        if (!svg_xml_internal_whitespace(byte_at(s, p)) && !starts(s, p, "?>", 2U)) {
            fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, p);
            return false;
        }
        if (starts(s, p, "?>", 2U)) {
            if (s->pending_declaration) { fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, p); return false; }
            s->pending_data = p;
            s->pending_first_nonspace = p;
            s->pending_pi_data_ready = true;
        } else {
            while (p < limit && p < s->input.len && svg_xml_internal_whitespace(byte_at(s, p))) ++p;
            if (p == limit && p < s->input.len) {
                s->pending_cursor = p;
                return true;
            }
            s->pending_data = p;
            s->pending_first_nonspace = p;
            s->pending_pi_data_ready = true;
        }
    }
    p = s->pending_data;
    while (p + 2U <= limit && p + 2U <= s->input.len && !starts(s, p, "?>", 2U)) ++p;
    if (p + 2U > limit && p + 2U <= s->input.len) {
        s->pending_data = p;
        s->pending_cursor = p;
        return true;
    }
    if (p + 2U > s->input.len) {
        fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, s->cursor);
        return false;
    }
    svg_xml_token token = {0};
    token.kind = s->pending_declaration ? SVG_XML_TOKEN_DECLARATION :
        SVG_XML_TOKEN_PROCESSING_INSTRUCTION;
    token.name = s->pending_name;
    token.value.offset = s->pending_first_nonspace;
    token.value.length = p - s->pending_first_nonspace;
    token.depth = s->depth;
    (void)span_at(s->cursor, p + 2U, &token.source);
    if (!s->pending_declaration && !count_node(s, s->cursor)) return false;
    if (!emit(s, token, s->cursor)) return false;
    size_t index = s->result.token_count - 1U;
    if (s->pending_declaration && !validate_decl_encoding(s, &s->tokens[index])) return false;
    s->cursor = p + 2U;
    stream_pending_clear(s);
    return true;
}

static bool stream_tag_complete(xml_scanner *s, size_t limit)
{
    if (s->pending_kind != XML_STREAM_PENDING_TAG || s->pending_start != s->cursor) {
        stream_pending_start(s, XML_STREAM_PENDING_TAG, s->cursor, s->cursor + 1U);
    }
    size_t p = s->pending_cursor;
    uint8_t quote = s->pending_quote;
    while (p < limit && p < s->input.len) {
        uint8_t c = byte_at(s, p);
        if (quote != 0U && c == quote) quote = 0U;
        else if (quote == 0U && (c == '\'' || c == '"')) quote = c;
        else if (quote == 0U && c == '>') {
            stream_pending_clear(s);
            return true;
        }
        ++p;
    }
    s->pending_cursor = p;
    s->pending_quote = quote;
    if (p == s->input.len) fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, s->cursor);
    return false;
}

