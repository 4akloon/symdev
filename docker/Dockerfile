# M0 toolchain image (spec §11). Same steps as
# docs/research/m0-bare-metal-runbook.md — Unknowns stay comments.
# Do not COPY SDK or ROM. No EKA2L1 in this image.
FROM ubuntu:24.04

# --- 1. Host packages ---
# Status: UNKNOWN — requires experiment
# Ubuntu 24.04 packages needed to *build* fedor4ever GCC are not recorded.
# Do not invent an apt list. Copy from GCC4Symbian docs during experiment.
# See docs/research/pipeline-and-tools.md and runbook ch.1.

# --- 2. GCC ---
# Status: UNKNOWN — requires experiment
# Need arm-none-symbianelf-g++ and arm-none-symbianelf-ld from fedor4ever.
# Plan-time sources (do not invent clone/wget argv):
#   https://github.com/fedor4ever/GCC4Symbian (build-toolchain.sh)
#   https://sourceforge.net/projects/gcce4symbian/files/GCC-14.2.0_BINUTILS-2.29.1/
#   https://fedor4ever.wordpress.com/2024/09/22/gcc-14-1-0-for-symbian-out/
# Clone commit, linux* branch name, build-toolchain.sh argv, and
# source vs SourceForge tarball: UNKNOWN. Copy from those docs during experiment.

# --- 3. elf2e32 ---
# Status: UNKNOWN — requires experiment
# C++ port (fedor4ever). How to obtain/build on Linux is Unknown.
# Plan-time sources:
#   https://github.com/fedor4ever/elf2e32_next (tested S60_3rd_FP2_SDK_v1.1)
#   https://github.com/fedor4ever/elf2e32 (older C++14 port)
# Do not vendor elf2e32 into this tree (EPL-1.0). Do not invent build argv.

# --- 4. SIS tools ---
# Status: UNKNOWN — Needs experiment
# Need makesis, signsis, makekeys.
# Native Linux binaries vs Wine: Needs experiment. Do not invent Wine argv.

# --- 5. SDK / ROM / EKA2L1 ---
# SDK is user-supplied. Bind-mount at /opt/symbian/sdk (see README).
# Never COPY, curl, or bake SDK or ROM into this image.
# No EKA2L1 in the image (GPL-3.0; user-installed; ROM cannot be in the image).
#
# docker run --rm -it \
#   -v /path/to/your/sdk:/opt/symbian/sdk \
#   -e EPOCROOT=/opt/symbian/sdk \
#   <image>
#
# EPOCROOT: Symbian tools historically want a trailing backslash.
# Whether a trailing / works on Linux is Needs experiment (runbook / spec §10.1).
ENV EPOCROOT=/opt/symbian/sdk
