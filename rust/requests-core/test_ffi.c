#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "requests.h"

int main() {
    printf("Testing Rust FFI...\n");

    requests_response* resp = requests_get("https://httpbin.org/get");
    if (!resp) {
        fprintf(stderr, "Request failed\n");
        return 1;
    }

    printf("Status: %d\n", resp->status_code);
    printf("URL: %s\n", resp->url);
    printf("Content length: %zu\n", resp->content_len);

    // Print first 100 bytes of content
    if (resp->content && resp->content_len > 0) {
        printf("Content preview: ");
        for (size_t i = 0; i < resp->content_len && i < 100; i++) {
            putchar(resp->content[i]);
        }
        printf("\n");
    }

    requests_free_response(resp);
    printf("FFI test passed!\n");
    return 0;
}
