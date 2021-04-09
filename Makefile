all: $(addsuffix .elf, $(shell find  -mindepth 2 -maxdepth 2 -name 'Cargo.toml' -printf "%h\n"))

include ./config.mk

TARGET_CFLAGS += -std=c99 -I ${SKE_PATH}/include

SKE_LIB := ${SKE_PATH}/lib/libxng.a \
           ${SKE_PATH}/lib/libpee.a

# build rust partitions
%.elf: %
	${TARGET_CC} ${TARGET_LDFLAGS} -o $@ $(SKE_LIB) \
		 $(shell find -name lib$^.a -printf '%T+ %p\n' | sort --reverse | awk 'FNR == 1 {print $$2}') \
		-lrt -ldl -lm -lpthread

clean:
	rm -f *.${TARGET}
	rm -f *.elf
	cargo clean
