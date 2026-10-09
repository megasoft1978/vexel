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

static bool stream_name(xml_scanner *s, size_t *position, size_t limit,
                       svg_xml_span *name, svg_xml_span *prefix, svg_xml_span *local,
                       bool *complete)
{
    if (!s->stream_name_active) {
        s->stream_name_active = true;
        s->stream_name_start = *position;
        s->stream_name_pos = *position;
        s->stream_name_first_colon = SIZE_MAX;
    }
    size_t p = s->stream_name_pos;
    bool first = p == s->stream_name_start;
    while (p < limit && p < s->input.len) {
        uint8_t lead = byte_at(s, p);
        size_t width = lead < 0x80U ? 1U :
            (lead >= 0xc2U && lead <= 0xdfU) ? 2U :
            (lead >= 0xe0U && lead <= 0xefU) ? 3U : 4U;
        if (width > limit - p && limit < s->input.len) break;
        size_t next = p;
        uint32_t cp = 0U;
        if (!svg_xml_internal_utf8_next(s->input.ptr, s->input.len, &next, &cp)) {
            fail(s, SVG_XML_ERROR_INVALID_UTF8, p);
            return false;
        }
        bool allowed = first ? svg_xml_internal_name_start(cp) : svg_xml_internal_name_char(cp);
        if (!allowed) {
            if (first) { fail(s, SVG_XML_ERROR_INVALID_NAME, p); return false; }
            break;
        }
        if (cp == ':' && s->stream_name_first_colon == SIZE_MAX)
            s->stream_name_first_colon = p;
        p = next;
        first = false;
        if (p - s->stream_name_start > XML_MAX_NAME_BYTES) {
            fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, s->stream_name_start);
            return false;
        }
    }
    s->stream_name_pos = p;
    if (first) {
        if (p == s->input.len) { fail(s, SVG_XML_ERROR_INVALID_NAME, s->stream_name_start); return false; }
        *position = p;
        *complete = false;
        return true;
    }
    if (p == limit && p < s->input.len) {
        *position = p;
        *complete = false;
        return true;
    }
    if (byte_at(s, p - 1U) == ':') { fail(s, SVG_XML_ERROR_INVALID_NAME, s->stream_name_start); return false; }
    name->offset = s->stream_name_start;
    name->length = p - s->stream_name_start;
    if (s->stream_name_first_colon == SIZE_MAX) {
        prefix->offset = s->stream_name_start;
        prefix->length = 0U;
        *local = *name;
    } else {
        prefix->offset = s->stream_name_start;
        prefix->length = s->stream_name_first_colon - s->stream_name_start;
        local->offset = s->stream_name_first_colon + 1U;
        local->length = p - local->offset;
    }
    *position = p;
    s->stream_name_active = false;
    *complete = true;
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

static bool stream_finish_start(xml_scanner *s, size_t end, bool empty)
{
    svg_xml_token *token = &s->tokens[s->stream_start_token_index];
    token->source.length = end - s->cursor;
    if (empty) token->kind = SVG_XML_TOKEN_EMPTY_ELEMENT;
    else {
        s->stack[s->depth].span = s->stream_start_name;
        ++s->depth;
    }
    ++s->result.element_count;
    s->cursor = end;
    s->stream_start_active = false;
    s->stream_start_name_ready = false;
    s->stream_start_phase = 0U;
    return true;
}

static bool stream_parse_start(xml_scanner *s, size_t limit)
{
    size_t p = s->stream_start_active ? s->stream_start_pos : s->cursor + 1U;
    if (!s->stream_start_active) {
        s->stream_start_active = true;
        s->stream_start_pos = p;
    }
    if (!s->stream_start_name_ready) {
        bool name_complete = false;
        if (!stream_name(s, &p, limit, &s->stream_start_name, &s->stream_start_prefix,
                         &s->stream_start_local_name, &name_complete)) return false;
        if (!name_complete) { s->stream_start_pos = p; return true; }
        if (!valid_qname(s, s->stream_start_prefix, s->stream_start_local_name)) return false;
        if (s->depth == 0U) {
            if (s->root_count != 0U) { fail(s, SVG_XML_ERROR_MULTIPLE_ROOTS, s->cursor); return false; }
            ++s->root_count;
        }
        if (s->depth >= s->options.limits.nesting_depth || s->depth >= XML_MAX_DEPTH) {
            fail(s, SVG_XML_ERROR_DEPTH_LIMIT, s->cursor);
            return false;
        }
        if (!count_node(s, s->cursor)) return false;
        svg_xml_token token = {0};
        token.kind = SVG_XML_TOKEN_START_ELEMENT;
        token.name = s->stream_start_name;
        token.prefix = s->stream_start_prefix;
        token.local_name = s->stream_start_local_name;
        token.depth = s->depth;
        (void)span_at(s->cursor, p, &token.source);
        if (!emit(s, token, s->cursor)) return false;
        s->stream_start_token_index = s->result.token_count - 1U;
        s->stream_start_local_attributes = 0U;
        s->stream_start_separated = false;
        s->stream_start_phase = 0U;
        s->stream_start_name_ready = true;
    }
    while (p < limit && p < s->input.len) {
        if (s->stream_start_phase == 0U) {
            while (p < limit && p < s->input.len && svg_xml_internal_whitespace(byte_at(s, p))) {
                ++p;
                s->stream_start_separated = true;
            }
            if (p >= limit && p < s->input.len) break;
            if (p >= s->input.len) break;
            if (byte_at(s, p) == '>') return stream_finish_start(s, p + 1U, false);
            if (byte_at(s, p) == '/' && p + 1U < limit && byte_at(s, p + 1U) == '>')
                return stream_finish_start(s, p + 2U, true);
            if (byte_at(s, p) == '/' && p + 1U >= limit && p + 1U < s->input.len) break;
            if (!s->stream_start_separated) { fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, p); return false; }
            if (s->stream_start_local_attributes >= s->options.limits.attributes_per_element ||
                s->total_attributes >= s->options.limits.total_attributes) {
                fail(s, SVG_XML_ERROR_ATTRIBUTE_LIMIT, p);
                return false;
            }
            s->stream_start_attribute_offset = p;
            s->stream_start_phase = 4U;
        } else if (s->stream_start_phase == 4U) {
            bool name_complete = false;
            if (!stream_name(s, &p, limit, &s->stream_attribute_name,
                             &s->stream_attribute_prefix, &s->stream_attribute_local_name,
                             &name_complete)) return false;
            if (!name_complete) break;
            if (!valid_qname(s, s->stream_attribute_prefix, s->stream_attribute_local_name)) return false;
            for (size_t i = s->stream_start_token_index + 1U; i < s->result.token_count; ++i) {
                if (s->tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE &&
                    qname_equal(s, s->stream_attribute_name, s->tokens[i].name)) {
                    fail(s, SVG_XML_ERROR_DUPLICATE_ATTRIBUTE, s->stream_start_attribute_offset);
                    return false;
                }
            }
            s->stream_start_phase = 1U;
        } else if (s->stream_start_phase == 1U) {
            while (p < limit && p < s->input.len && svg_xml_internal_whitespace(byte_at(s, p))) ++p;
            if (p >= limit && p < s->input.len) break;
            if (p >= s->input.len || byte_at(s, p) != '=') { fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, p); return false; }
            ++p;
            s->stream_start_phase = 2U;
        } else if (s->stream_start_phase == 2U) {
            while (p < limit && p < s->input.len && svg_xml_internal_whitespace(byte_at(s, p))) ++p;
            if (p >= limit && p < s->input.len) break;
            if (p >= s->input.len || (byte_at(s, p) != '\'' && byte_at(s, p) != '"')) {
                fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, p);
                return false;
            }
            s->stream_start_quote = byte_at(s, p++);
            s->stream_start_value_offset = p;
            s->stream_start_phase = 3U;
        } else {
            bool paused = false;
            while (p < limit && p < s->input.len && byte_at(s, p) != s->stream_start_quote) {
                if (byte_at(s, p) == '<') { fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, p); return false; }
                if (byte_at(s, p) == '&') {
                    size_t semi = p;
                    while (semi < limit && semi < s->input.len && byte_at(s, semi) != ';') ++semi;
                    if (semi == limit && semi < s->input.len) { paused = true; break; }
                    if (!scan_reference(s, &p)) return false;
                } else ++p;
            }
            if (paused) break;
            if (p >= limit && p < s->input.len) break;
            if (p >= s->input.len) { fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, s->stream_start_value_offset - 1U); return false; }
            svg_xml_token attribute = {0};
            attribute.kind = SVG_XML_TOKEN_ATTRIBUTE;
            attribute.name = s->stream_attribute_name;
            attribute.prefix = s->stream_attribute_prefix;
            attribute.local_name = s->stream_attribute_local_name;
            attribute.depth = s->depth;
            (void)span_at(s->stream_start_attribute_offset, p + 1U, &attribute.source);
            (void)span_at(s->stream_start_value_offset, p, &attribute.value);
            if (!emit(s, attribute, s->stream_start_attribute_offset)) return false;
            ++s->stream_start_local_attributes;
            ++s->total_attributes;
            ++s->result.attribute_count;
            ++p;
            s->stream_start_separated = false;
            s->stream_start_phase = 0U;
        }
    }
    s->stream_start_pos = p;
    if (p == s->input.len) fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX,
        s->stream_start_phase == 3U ? s->stream_start_value_offset - 1U : s->cursor);
    return s->result.diagnostic.category == SVG_XML_ERROR_NONE;
}

static bool stream_parse_end(xml_scanner *s, size_t limit)
{
    size_t p = s->stream_end_pos;
    svg_xml_span name = s->stream_end_name, prefix = s->stream_end_prefix;
    svg_xml_span local = s->stream_end_local_name;
    if (!s->stream_end_active) {
        s->stream_end_active = true;
        s->stream_end_prefix_progress = 0U;
        s->stream_end_pos = s->cursor;
        p = s->cursor;
    }
    while (s->stream_end_prefix_progress < 2U && p < limit) {
        uint8_t expected = s->stream_end_prefix_progress == 0U ? '<' : '/';
        if (byte_at(s, p) != expected) { fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, p); return false; }
        ++p;
        ++s->stream_end_prefix_progress;
        s->stream_end_pos = p;
    }
    if (s->stream_end_prefix_progress < 2U) return true;
    if (!s->stream_end_name_ready) {
        bool name_complete = false;
        if (!stream_name(s, &p, limit, &name, &prefix, &local, &name_complete)) return false;
        if (!name_complete) { s->stream_end_pos = p; return true; }
        if (s->depth == 0U || !qname_equal(s, name, s->stack[s->depth - 1U].span)) {
            fail(s, SVG_XML_ERROR_MISMATCHED_TAG, s->cursor);
            return false;
        }
        s->stream_end_name = name;
        s->stream_end_prefix = prefix;
        s->stream_end_local_name = local;
        s->stream_end_name_ready = true;
    }
    while (p < limit && p < s->input.len && svg_xml_internal_whitespace(byte_at(s, p))) ++p;
    if (p >= limit && p < s->input.len) { s->stream_end_pos = p; return true; }
    if (p >= s->input.len || byte_at(s, p) != '>') {
        fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, p);
        return false;
    }
    svg_xml_token token = {0};
    token.kind = SVG_XML_TOKEN_END_ELEMENT;
    token.name = name;
    token.prefix = prefix;
    token.local_name = local;
    token.depth = s->depth - 1U;
    (void)span_at(s->cursor, p + 1U, &token.source);
    if (!emit(s, token, s->cursor)) return false;
    --s->depth;
    s->cursor = p + 1U;
    s->stream_end_active = false;
    s->stream_end_name_ready = false;
    s->stream_end_prefix_progress = 0U;
    return true;
}
