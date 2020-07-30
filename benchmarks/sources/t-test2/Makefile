TARGET = build

.PHONY: build build_debug
build:
	$(CC) --std=c99 -c t-test2.c -I../common

build_debug:
	$(CC) --std=c99 -g -c t-test2.c -I../common

clean:
	rm -f CC-*
