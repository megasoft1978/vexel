#include "platform_media.h"

const char *platform_media_status_name(platform_media_status status)
{
    switch (status) {
    case PLATFORM_MEDIA_OK: return "ok";
    case PLATFORM_MEDIA_INVALID_ARGUMENT: return "invalid-argument";
    case PLATFORM_MEDIA_INVALID_UTF8: return "invalid-utf8";
    case PLATFORM_MEDIA_LIMIT: return "limit";
    case PLATFORM_MEDIA_UNSUPPORTED: return "unsupported";
    case PLATFORM_MEDIA_ANIMATED: return "animated";
    case PLATFORM_MEDIA_CORRUPT: return "corrupt";
    case PLATFORM_MEDIA_OUT_OF_MEMORY: return "out-of-memory";
    }
    return "unknown";
}
