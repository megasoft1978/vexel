#ifndef VEXEL_PLATFORM_MEDIA_H
#define VEXEL_PLATFORM_MEDIA_H

typedef enum platform_media_status {
    PLATFORM_MEDIA_OK = 0,              /* operation completed */
    PLATFORM_MEDIA_INVALID_ARGUMENT,    /* invalid slice, request, or output */
    PLATFORM_MEDIA_INVALID_UTF8,         /* malformed UTF-8 input */
    PLATFORM_MEDIA_LIMIT,                /* a documented byte/count/size cap */
    PLATFORM_MEDIA_UNSUPPORTED,          /* valid input, unsupported feature */
    PLATFORM_MEDIA_ANIMATED,             /* multi-frame image input */
    PLATFORM_MEDIA_CORRUPT,              /* malformed or truncated image data */
    PLATFORM_MEDIA_OUT_OF_MEMORY         /* allocation/framework resource failure */
} platform_media_status;

const char *platform_media_status_name(platform_media_status status);

#endif
