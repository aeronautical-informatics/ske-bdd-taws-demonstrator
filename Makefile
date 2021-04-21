include ./config.mk

all: $(addsuffix .elf, $(PARTITION_NAMES))

TARGET_CFLAGS += -std=c99

SKE_LIB := ${SKE_PATH}/lib/libxng.a \
           ${SKE_PATH}/lib/libpee.a

# build rust partitions
%.elf: %
	${TARGET_CC} ${TARGET_LDFLAGS} -o $@ $(SKE_LIB) \
		 $(shell find $(PARTITIONS_ROOT) -name lib$^.a -printf '%T+ %p\n' | sort --reverse | awk 'FNR == 1 {print $$2}') \
		-lrt -ldl -lm -lpthread

check: all
	ske-rs run 1e3

clean:
	rm -f *.${TARGET}
	rm -f *.elf
