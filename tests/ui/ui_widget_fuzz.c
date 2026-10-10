#include "ui_widgets.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAX_FUZZ_BYTES UI_WIDGET_EDIT_MAX_BYTES

static uint32_t random_next(uint32_t *state)
{
    uint32_t value = *state;
    value ^= value << 13U;
    value ^= value >> 17U;
    value ^= value << 5U;
    *state = value == 0U ? UINT32_C(0x9e3779b9) : value;
    return *state;
}

static bool parse_argument(const char *text, uint32_t *out_value)
{
    if (text == NULL || out_value == NULL || text[0] == '\0') return false;
    char *end = NULL;
    unsigned long value = strtoul(text, &end, 10);
    if (end == text || *end != '\0' || value == 0UL || value > UINT32_MAX) return false;
    *out_value = (uint32_t)value;
    return true;
}

int main(int argc, char **argv)
{
    uint32_t cases = 200000U;
    uint32_t seed = UINT32_C(20261010);
    if ((argc > 3) || (argc > 1 && !parse_argument(argv[1], &cases)) ||
        (argc > 2 && !parse_argument(argv[2], &seed))) {
        (void)fprintf(stderr, "usage: ui_widget_fuzz [cases [seed]]\n");
        return 2;
    }
    uint32_t state = seed;
    uint8_t input[MAX_FUZZ_BYTES];
    char encoded[UI_WIDGET_EDIT_MAX_BYTES + 1U];
    uint64_t number_valid = 0U, hex_valid = 0U;
    for (uint32_t iteration = 0U; iteration < cases; ++iteration) {
        size_t length = (size_t)(random_next(&state) % (MAX_FUZZ_BYTES + 1U));
        for (size_t index = 0U; index < length; ++index)
            input[index] = (uint8_t)(random_next(&state) & 0xffU);
        if ((iteration & 15U) == 0U) {
            static const char *number_samples[] = {"0", "-1.25", "+.5", "6e-4", " 14.0 "};
            const char *number = number_samples[(iteration >> 4U) % 5U];
            length = strlen(number);
            memcpy(input, number, length);
        }
        double number = 0.0;
        if (ui_widget_number_parse((const char *)input, length, &number)) {
            number_valid++;
            if (!isfinite(number)) return 3;
            size_t output_length = 0U;
            if (!ui_widget_number_format(number, encoded, sizeof(encoded), &output_length)) return 4;
            double round_trip = 0.0;
            if (!ui_widget_number_parse(encoded, output_length, &round_trip) ||
                fabs(number - round_trip) > fmax(1.0, fabs(number)) * 1e-14) return 5;
        }
        uint32_t rgba = 0U;
        if (ui_widget_hex_parse((const char *)input, length, &rgba)) {
            hex_valid++;
            size_t output_length = 0U;
            if (!ui_widget_hex_format(rgba, encoded, sizeof(encoded), &output_length)) return 6;
            uint32_t round_trip = 0U;
            if (!ui_widget_hex_parse(encoded, output_length, &round_trip) || round_trip != rgba)
                return 7;
        }
        if ((iteration & 15U) == 0U) {
            static const char *hex_samples[] = {"#000", "#4f7c", "#12aBef", "#12345678"};
            const char *hex = hex_samples[(iteration >> 4U) % 4U];
            size_t hex_length = strlen(hex);
            if (ui_widget_hex_parse(hex, hex_length, &rgba)) hex_valid++;
        }
    }
    (void)printf("{\"cases\":%u,\"seed\":%u,\"number_valid\":%llu,\"hex_valid\":%llu}\n",
        cases, seed, (unsigned long long)number_valid, (unsigned long long)hex_valid);
    return 0;
}
