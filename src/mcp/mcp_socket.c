#define _POSIX_C_SOURCE 200809L
#define _DARWIN_C_SOURCE

#include "mcp_socket.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/un.h>
#include <unistd.h>

bool mcp_socket_path(char *out, size_t capacity)
{
    if (out == NULL || capacity == 0U || capacity > sizeof(((struct sockaddr_un *)0)->sun_path)) return false;
    const char *configured = getenv("VEXEL_MCP_SOCKET");
    if (configured != NULL && configured[0] != '\0') {
        size_t length = strnlen(configured, capacity);
        if (configured[0] != '/' || length == 0U || length >= capacity) return false;
        memcpy(out, configured, length + 1U);
        return true;
    }
    const char *temporary = getenv("TMPDIR");
    if (temporary == NULL || temporary[0] != '/') temporary = "/tmp";
    size_t prefix = strnlen(temporary, capacity);
    if (prefix == 0U || prefix >= capacity) return false;
    int amount = snprintf(out, capacity, "%s%svexel-mcp-%u.sock", temporary,
        temporary[prefix - 1U] == '/' ? "" : "/", (unsigned)getuid());
    return amount > 0 && (size_t)amount < capacity;
}
