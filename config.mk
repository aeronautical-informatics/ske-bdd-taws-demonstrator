# Configuration file
######################

# Location of SKE
SKE_PATH ?= ..

# You shouldn't need to modify the following
TARGET_CCPATH =
TARGET_CCPREFIX = 
TARGET_CFLAGS += -finstrument-functions
TARGET_ASFLAGS +=
TARGET_LDFLAGS +=

HOST = x86
HOST_CCPATH = 
HOST_CCPREFIX =
HOST_CFLAGS +=
HOST_ASFLAGS +=
HOST_LDFLAGS +=

TARGET_CC = ${TARGET_CCPATH}${TARGET_CCPREFIX}gcc 
TARGET_LD= ${TARGET_CCPATH}${TARGET_CCPREFIX}ld

# DO NOT MODIFY. ABSOLUTE path to the LTE distro. 
LTE_PATH=$(dir $(abspath $(lastword $(MAKEFILE_LIST))))/../

# DO NOT MODIFY. SKE target.
TARGET = skelinux
