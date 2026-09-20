#!/bin/bash
#
# Run the S60 3rd FP2 SDK's own build-file generator (its Perl programs) on a
# Linux host and capture the makefile it produces.  Nothing in the SDK tree is
# read-write: the generator sees a mirror whose build directory is the only
# writable part.  See experiment 63 in experiment-backlog.md.
#
#   sdk-generator.sh setup
#   sdk-generator.sh make <project-dir-holding-bld.inf> <MMP-BASENAME> [PLATFORM]
#
# Environment:
#   SDKGEN_SDK    the SDK to mirror        (default /home/genius/sdk/S60_3rd_FP2)
#   SDKGEN_WORK   scratch directory        (default /tmp/sdk-generator-work)
#   SDKGEN_GCCE   directory holding arm-none-symbianelf-g++, optional.  Without
#                 it the generator cannot resolve the tool-chain include
#                 directory or the linker's two -L paths and leaves them empty.
#
set -euo pipefail

SDK=${SDKGEN_SDK:-/home/genius/sdk/S60_3rd_FP2}
W=${SDKGEN_WORK:-/tmp/sdk-generator-work}
GCCE=${SDKGEN_GCCE:-}
HERE=$(cd "$(dirname "$0")" && pwd)

# The generator builds Windows-shaped paths throughout, so EPOCROOT has to be
# one: drive-less, backslash-separated, trailing separator.
winpath() { printf '%s' "$1" | tr '/' '\\'; }
EPOCROOT_WIN="$(winpath "$W/sdkroot")\\"

setup() {
    rm -rf "$W"
    mkdir -p "$W"/{shim,bin,gccbin/gcc/bin,perl-overlay/File,proj,out}

    # 1. The path shim.  Every libc call that takes a path gets backslashes
    #    turned into slashes and its spelling resolved case-insensitively;
    #    /bin/sh -c command lines get cmd.exe quoting turned into sh quoting;
    #    mkdir gets a mode the owner can actually enter, as on Windows.
    gcc -shared -fPIC -O2 -o "$W/shim/winpath.so" "$HERE/sdk-generator-winpath.c" -ldl

    # 2. A stand-in for the cmd.exe `set NAME` builtin.  perl execs this
    #    directly (the command has no shell metacharacters), so a PATH
    #    executable is enough.
    cat > "$W/bin/set" <<'EOF'
#!/bin/sh
env | grep "^$1"
EOF

    # 3. make with bash as its shell.  The generator extracts the tool-chain
    #    settings by running `echo VAR=$(VAR)` through make; /bin/sh would eat
    #    the backslashes of every path in those values.
    cat > "$W/bin/make" <<EOF
#!/bin/sh
exec $(command -v make) SHELL=/bin/bash "\$@"
EOF

    # 4. A stand-in for the SDK's Cygwin cpp.exe.  See the comments in the
    #    script itself for the three differences it has to paper over.
    cp "$HERE/sdk-generator-cpp.py" "$W/gccbin/gcc/bin/cpp.exe"
    ln -sf cpp.exe "$W/gccbin/gcc/bin/CPP.EXE"
    ln -sf cpp.exe "$W/gccbin/gcc/bin/cpp"

    # 5. Perl overlay: a Win32 stand-in (the source checker wants
    #    GetLongPathName, which is an 8.3 concept), a File::Path that knows "\"
    #    is a separator as it does on Windows, and a prelude that puts
    #    File::Basename into its Windows mode.
    cat > "$W/perl-overlay/Win32.pm" <<'EOF'
package Win32;
sub GetLongPathName { return $_[0]; }
1;
EOF
    cp "$HERE/sdk-generator-File-Path.pm" "$W/perl-overlay/File/Path.pm"
    cat > "$W/perl-overlay/SdkWin.pm" <<'EOF'
package SdkWin;
use File::Basename ();
File::Basename::fileparse_set_fstype('MSWin32');
1;
EOF

    # 6. The mirror.  epoc32/tools must be a real directory (the tools take
    #    their library path from the script's own directory, resolving symbolic
    #    links), so mirror it file by file; epoc32/build is real and writable.
    mkdir -p "$W/sdkroot/epoc32/tools" "$W/sdkroot/epoc32/build"
    for d in "$SDK"/epoc32/*; do
        [ "$(basename "$d")" = tools ] && continue
        ln -s "$d" "$W/sdkroot/epoc32/$(basename "$d")"
    done
    for f in "$SDK"/epoc32/tools/*; do
        ln -s "$f" "$W/sdkroot/epoc32/tools/$(basename "$f")"
    done

    # 7. The one patched SDK module.  `defined` applied to a hash was removed
    #    from perl in 5.22; both uses guard a duplicate-definition warning for
    #    platform specification files, so dropping `defined` leaves a plain
    #    truth test on the same hash and cannot change the generated makefile.
    rm -f "$W/sdkroot/epoc32/tools/e32plat.pm"
    cp "$SDK/epoc32/tools/e32plat.pm" "$W/sdkroot/epoc32/tools/e32plat.pm"
    chmod u+w "$W/sdkroot/epoc32/tools/e32plat.pm"
    perl -0pi -e 's/\bdefined (\%\{\$Plat\{\$(?:BSF|ASSP)\}\})/$1/g' \
        "$W/sdkroot/epoc32/tools/e32plat.pm"

    # 8. The launcher.
    cat > "$W/bin/sdkperl" <<EOF
#!/bin/sh
export LD_PRELOAD="$W/shim/winpath.so"
export PATH="$W/bin:$W/gccbin/gcc/bin:$W/sdkroot/epoc32/tools${GCCE:+:$GCCE}:\$PATH"
export EPOCROOT='$EPOCROOT_WIN'
# checkgcc.pm reads the cmd.exe-style Path and insists the first cpp it finds
# sits in a directory whose name ends in \\GCC\\BIN\\.
export Path='$(winpath "$W/gccbin/gcc/bin")\\'
export PERL5LIB="$W/perl-overlay"
export PERL5OPT="-MSdkWin"
exec perl -I"$W/perl-overlay" -I"$W/sdkroot/epoc32/tools" "\$@"
EOF

    chmod +x "$W/bin/set" "$W/bin/make" "$W/bin/sdkperl" "$W/gccbin/gcc/bin/cpp.exe"
    echo "work directory ready: $W"
}

generate() {
    local src=$1 base=$2 plat=${3:-GCCE}
    local name dst grp
    name=$(basename "$src")
    dst=$W/proj/$name
    rm -rf "$dst"
    cp -r "$src" "$dst"
    chmod -R u+w "$dst"

    # The generator is started from the directory holding bld.inf, which in a
    # Symbian project is conventionally group/ but need not be.
    grp=$(find "$dst" -iname bld.inf -printf '%h\n' | sort | head -1)
    if [ -z "$grp" ]; then
        echo "no bld.inf under $dst" >&2
        exit 1
    fi

    ( cd "$grp" && "$W/bin/sdkperl" "$W/sdkroot/epoc32/tools/bldmake.pl" bldfiles )
    ( cd "$grp" && "$W/bin/sdkperl" "$W/sdkroot/epoc32/tools/makmake.pl" \
        -D "$(winpath "$grp")\\$base" "$plat" )

    find "$W/sdkroot/epoc32/build" -name "$base.$plat" -print
}

case ${1:-} in
    setup) setup ;;
    make)  shift; generate "$@" ;;
    *) echo "usage: $0 setup | $0 make <project-dir> <MMP-BASENAME> [PLATFORM]" >&2
       exit 2 ;;
esac
