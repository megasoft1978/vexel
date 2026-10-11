#include "app/welcome_model.h"

#include <stdio.h>
#include <string.h>

static bool require(bool condition, const char *message)
{
    if (condition) return true;
    (void)fprintf(stderr, "welcome model: %s\n", message);
    return false;
}

static bool test_path_contract(void)
{
    char maximum[WELCOME_RECENT_PATH_CAP];
    memset(maximum, 'a', sizeof(maximum));
    maximum[sizeof(maximum) - 5U] = '.';
    memcpy(maximum + sizeof(maximum) - 4U, "svg", 3U);
    maximum[sizeof(maximum) - 1U] = '\0';
    const char embedded_nul[] = {'a', 'b', '\0', '.', 's', 'v', 'g'};
    const char *upper = "/tmp/vector.SVG";
    const char *with_nul = "/tmp/vector.svg";
    const char *wrong_suffix = "/tmp/vector.svgx";
    return require(welcome_recent_path_supported(upper, strlen(upper)),
            "accepts an uppercase SVG suffix") &&
        require(welcome_recent_path_supported(maximum, sizeof(maximum) - 1U),
            "accepts the largest terminated path") &&
        require(!welcome_recent_path_supported(wrong_suffix, strlen(wrong_suffix)),
            "rejects a non-SVG suffix") &&
        require(!welcome_recent_path_supported(with_nul, strlen(with_nul) + 1U),
            "rejects an embedded terminator in the slice") &&
        require(!welcome_recent_path_supported(embedded_nul, sizeof(embedded_nul)),
            "rejects embedded NUL bytes") &&
        require(!welcome_recent_path_supported(maximum, sizeof(maximum)),
            "rejects an unterminated oversized path");
}

static bool test_recent_order_and_bound(void)
{
    welcome_recent_list list = {0};
    for (size_t index = 0U; index < WELCOME_RECENT_LIMIT + 1U; ++index) {
        char path[64];
        int amount = snprintf(path, sizeof(path), "/tmp/file-%zu.svg", index);
        if (amount < 0 || (size_t)amount >= sizeof(path) ||
            !welcome_recent_add(&list, path, (size_t)amount)) return false;
    }
    welcome_recent_view view;
    if (!require(list.count == WELCOME_RECENT_LIMIT,
            "caps the list at eight items") ||
        !require(welcome_recent_get(&list, 0U, &view) &&
            view.path_length == strlen("/tmp/file-8.svg") &&
            memcmp(view.path, "/tmp/file-8.svg", view.path_length) == 0,
            "places the newest path first") ||
        !require(welcome_recent_get(&list, WELCOME_RECENT_LIMIT - 1U, &view) &&
            memcmp(view.path, "/tmp/file-1.svg", view.path_length) == 0,
            "drops the oldest path when full")) return false;
    if (!welcome_recent_add(&list, "/tmp/file-3.svg", strlen("/tmp/file-3.svg")) ||
        !require(list.count == WELCOME_RECENT_LIMIT,
            "does not duplicate an existing path") ||
        !require(welcome_recent_get(&list, 0U, &view) &&
            memcmp(view.path, "/tmp/file-3.svg", view.path_length) == 0,
            "moves a duplicate to the front")) return false;
    return true;
}

static bool test_views_and_mutation(void)
{
    welcome_recent_list list = {0};
    const char *path = "/Users/artist/Maps/river.svg";
    if (!welcome_recent_add(&list, path, strlen(path))) return false;
    welcome_recent_view view;
    if (!welcome_recent_get(&list, 0U, &view) ||
        !require(view.name_length == strlen("river.svg") &&
            memcmp(view.name, "river.svg", view.name_length) == 0,
            "exposes the basename") ||
        !require(view.parent_length == strlen("/Users/artist/Maps") &&
            memcmp(view.parent, "/Users/artist/Maps", view.parent_length) == 0,
            "exposes the parent folder") ||
        !require(!welcome_recent_get(&list, 1U, &view),
            "rejects an out-of-range row") ||
        !require(welcome_recent_remove(&list, path, strlen(path)),
            "removes an exact path") ||
        !require(!welcome_recent_remove(&list, path, strlen(path)),
            "reports absent paths") ||
        !require(list.count == 0U, "removal updates the row count")) return false;
    welcome_recent_clear(&list);
    return require(list.count == 0U, "clear leaves an empty list");
}

static bool test_utf8_path_views_remain_byte_bounded(void)
{
    static const char path[] = "/Users/artiste/Créations/étoile.svg";
    welcome_recent_list list = {0};
    welcome_recent_view view;
    if (!welcome_recent_add(&list, path, sizeof(path) - 1U) ||
        !welcome_recent_get(&list, 0U, &view)) return false;
    return require(view.name_length == strlen("étoile.svg") &&
            memcmp(view.name, "étoile.svg", view.name_length) == 0,
            "keeps UTF-8 basename lengths in bytes") &&
        require(view.parent_length == strlen("/Users/artiste/Créations") &&
            memcmp(view.parent, "/Users/artiste/Créations", view.parent_length) == 0,
            "keeps UTF-8 parent lengths in bytes");
}

int main(void)
{
    if (!test_path_contract() || !test_recent_order_and_bound() ||
        !test_views_and_mutation() || !test_utf8_path_views_remain_byte_bounded()) return 1;
    (void)puts("welcome model: bounded recents passed");
    return 0;
}
