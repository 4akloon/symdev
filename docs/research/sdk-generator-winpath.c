/*
 * winpath.so -- LD_PRELOAD path shim for running the Symbian SDK's Perl
 * build-file generator on Linux.
 *
 * It only touches the operating-system boundary: every libc call that takes a
 * path gets the path rewritten as
 *   1. backslash -> forward slash, and
 *   2. component-by-component case-insensitive resolution, used only when the
 *      literal spelling does not exist.
 * Nothing else is changed; the Perl code sees exactly the strings it built.
 */

#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <dirent.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdarg.h>
#include <utime.h>
#include <sys/time.h>

#define WP_MAX 4096

static __thread int wp_busy = 0;

static int (*r_lstat)(const char *, struct stat *);
static DIR *(*r_opendir)(const char *);
static struct dirent *(*r_readdir)(DIR *);
static int (*r_closedir)(DIR *);

static void wp_init(void)
{
    if (r_lstat)
        return;
    r_lstat = dlsym(RTLD_NEXT, "lstat");
    r_opendir = dlsym(RTLD_NEXT, "opendir");
    r_readdir = dlsym(RTLD_NEXT, "readdir");
    r_closedir = dlsym(RTLD_NEXT, "closedir");
}

static int wp_exists(const char *p)
{
    struct stat st;
    return r_lstat(p, &st) == 0;
}

/* Case-insensitively resolve `in` against the real filesystem into `out`. */
static void wp_resolve(const char *in, char *out, size_t outsz)
{
    char work[WP_MAX];
    size_t olen = 0;
    const char *p;
    int broken = 0;

    if (strlen(in) >= sizeof(work)) {
        snprintf(out, outsz, "%s", in);
        return;
    }
    snprintf(work, sizeof(work), "%s", in);

    p = work;
    out[0] = '\0';
    if (*p == '/') {
        out[olen++] = '/';
        out[olen] = '\0';
        while (*p == '/')
            p++;
    }

    while (*p) {
        const char *slash = strchr(p, '/');
        size_t clen = slash ? (size_t)(slash - p) : strlen(p);
        char comp[NAME_MAX + 1];
        char cand[WP_MAX];

        if (clen > NAME_MAX)
            clen = NAME_MAX;
        memcpy(comp, p, clen);
        comp[clen] = '\0';

        if (!broken && clen && strcmp(comp, ".") && strcmp(comp, "..")) {
            snprintf(cand, sizeof(cand), "%s%s", out, comp);
            if (!wp_exists(cand)) {
                /* look for a spelling that differs only by case */
                char dirbuf[WP_MAX];
                DIR *d;
                struct dirent *e;
                int found = 0;

                if (olen == 0)
                    snprintf(dirbuf, sizeof(dirbuf), ".");
                else
                    snprintf(dirbuf, sizeof(dirbuf), "%s", out);
                d = r_opendir(dirbuf);
                if (d) {
                    while ((e = r_readdir(d)) != NULL) {
                        if (!strcasecmp(e->d_name, comp)) {
                            snprintf(comp, sizeof(comp), "%s", e->d_name);
                            found = 1;
                            break;
                        }
                    }
                    r_closedir(d);
                }
                if (!found)
                    broken = 1; /* nothing below this can be resolved either */
            }
        }

        if (clen) {
            size_t need = strlen(comp);
            if (olen + need + 2 >= outsz) {
                snprintf(out, outsz, "%s", in);
                return;
            }
            memcpy(out + olen, comp, need);
            olen += need;
            out[olen] = '\0';
        }
        if (slash) {
            if (olen + 2 < outsz) {
                out[olen++] = '/';
                out[olen] = '\0';
            }
            p = slash + 1;
            while (*p == '/')
                p++;
        } else {
            p += strlen(p);
        }
    }
    if (olen == 0)
        snprintf(out, outsz, "%s", in);
}

/* Returns either `path` unchanged or a pointer into `buf`. */
static const char *wp_fix(const char *path, char *buf, size_t bufsz)
{
    char slashed[WP_MAX];
    size_t i, n;

    if (!path || !*path)
        return path;
    wp_init();
    if (wp_busy)
        return path;

    n = strlen(path);
    if (n >= sizeof(slashed))
        return path;

    wp_busy = 1;
    memcpy(slashed, path, n + 1);
    for (i = 0; i < n; i++)
        if (slashed[i] == '\\')
            slashed[i] = '/';

    if (!memcmp(slashed, path, n + 1) && wp_exists(path)) {
        wp_busy = 0;
        return path; /* exact spelling already works */
    }
    if (wp_exists(slashed)) {
        wp_busy = 0;
        snprintf(buf, bufsz, "%s", slashed);
        return buf;
    }
    wp_resolve(slashed, buf, bufsz);
    wp_busy = 0;
    return buf;
}

#define FIX(p) char wp_b[WP_MAX]; const char *wp_p = wp_fix((p), wp_b, sizeof(wp_b))

/* ------------------------------------------------------------------ */

int open(const char *path, int flags, ...)
{
    static int (*real)(const char *, int, ...);
    mode_t mode = 0;
    va_list ap;
    if (!real) real = dlsym(RTLD_NEXT, "open");
    va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    { FIX(path); return real(wp_p, flags, mode); }
}

int open64(const char *path, int flags, ...)
{
    static int (*real)(const char *, int, ...);
    mode_t mode = 0;
    va_list ap;
    if (!real) real = dlsym(RTLD_NEXT, "open64");
    va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    { FIX(path); return real(wp_p, flags, mode); }
}

int openat(int fd, const char *path, int flags, ...)
{
    static int (*real)(int, const char *, int, ...);
    mode_t mode = 0;
    va_list ap;
    if (!real) real = dlsym(RTLD_NEXT, "openat");
    va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    { FIX(path); return real(fd, wp_p, flags, mode); }
}

FILE *fopen(const char *path, const char *m)
{
    static FILE *(*real)(const char *, const char *);
    if (!real) real = dlsym(RTLD_NEXT, "fopen");
    { FIX(path); return real(wp_p, m); }
}

FILE *fopen64(const char *path, const char *m)
{
    static FILE *(*real)(const char *, const char *);
    if (!real) real = dlsym(RTLD_NEXT, "fopen64");
    { FIX(path); return real(wp_p, m); }
}

FILE *freopen(const char *path, const char *m, FILE *s)
{
    static FILE *(*real)(const char *, const char *, FILE *);
    if (!real) real = dlsym(RTLD_NEXT, "freopen");
    { FIX(path); return real(wp_p, m, s); }
}

int stat(const char *path, struct stat *b)
{
    static int (*real)(const char *, struct stat *);
    if (!real) real = dlsym(RTLD_NEXT, "stat");
    { FIX(path); return real(wp_p, b); }
}

int stat64(const char *path, struct stat64 *b)
{
    static int (*real)(const char *, struct stat64 *);
    if (!real) real = dlsym(RTLD_NEXT, "stat64");
    { FIX(path); return real(wp_p, b); }
}

int lstat(const char *path, struct stat *b)
{
    static int (*real)(const char *, struct stat *);
    if (!real) real = dlsym(RTLD_NEXT, "lstat");
    { FIX(path); return real(wp_p, b); }
}

int lstat64(const char *path, struct stat64 *b)
{
    static int (*real)(const char *, struct stat64 *);
    if (!real) real = dlsym(RTLD_NEXT, "lstat64");
    { FIX(path); return real(wp_p, b); }
}

int __xstat(int v, const char *path, struct stat *b)
{
    static int (*real)(int, const char *, struct stat *);
    if (!real) real = dlsym(RTLD_NEXT, "__xstat");
    { FIX(path); return real(v, wp_p, b); }
}

int __lxstat(int v, const char *path, struct stat *b)
{
    static int (*real)(int, const char *, struct stat *);
    if (!real) real = dlsym(RTLD_NEXT, "__lxstat");
    { FIX(path); return real(v, wp_p, b); }
}

int fstatat(int fd, const char *path, struct stat *b, int f)
{
    static int (*real)(int, const char *, struct stat *, int);
    if (!real) real = dlsym(RTLD_NEXT, "fstatat");
    { FIX(path); return real(fd, wp_p, b, f); }
}

int statx(int fd, const char *restrict path, int f, unsigned int m,
          struct statx *restrict b)
{
    static int (*real)(int, const char *restrict, int, unsigned int,
                       struct statx *restrict);
    if (!real) real = dlsym(RTLD_NEXT, "statx");
    { FIX(path); return real(fd, wp_p, f, m, b); }
}

int access(const char *path, int m)
{
    static int (*real)(const char *, int);
    if (!real) real = dlsym(RTLD_NEXT, "access");
    { FIX(path); return real(wp_p, m); }
}

int faccessat(int fd, const char *path, int m, int f)
{
    static int (*real)(int, const char *, int, int);
    if (!real) real = dlsym(RTLD_NEXT, "faccessat");
    { FIX(path); return real(fd, wp_p, m, f); }
}

int euidaccess(const char *path, int m)
{
    static int (*real)(const char *, int);
    if (!real) real = dlsym(RTLD_NEXT, "euidaccess");
    { FIX(path); return real(wp_p, m); }
}

DIR *opendir(const char *path)
{
    static DIR *(*real)(const char *);
    if (!real) real = dlsym(RTLD_NEXT, "opendir");
    { FIX(path); return real(wp_p); }
}

int mkdir(const char *path, mode_t m)
{
    static int (*real)(const char *, mode_t);
    if (!real) real = dlsym(RTLD_NEXT, "mkdir");
    /* Windows ignores the mode argument of mkdir, so a directory created
     * there is always usable by its owner.  One SDK module asks for mode 2,
     * which on Linux yields a directory the process cannot enter. */
    m |= 0700;
    { FIX(path); return real(wp_p, m); }
}

int rmdir(const char *path)
{
    static int (*real)(const char *);
    if (!real) real = dlsym(RTLD_NEXT, "rmdir");
    { FIX(path); return real(wp_p); }
}

int unlink(const char *path)
{
    static int (*real)(const char *);
    if (!real) real = dlsym(RTLD_NEXT, "unlink");
    { FIX(path); return real(wp_p); }
}

int chdir(const char *path)
{
    static int (*real)(const char *);
    if (!real) real = dlsym(RTLD_NEXT, "chdir");
    { FIX(path); return real(wp_p); }
}

int chmod(const char *path, mode_t m)
{
    static int (*real)(const char *, mode_t);
    if (!real) real = dlsym(RTLD_NEXT, "chmod");
    { FIX(path); return real(wp_p, m); }
}

int utime(const char *path, const struct utimbuf *t)
{
    static int (*real)(const char *, const struct utimbuf *);
    if (!real) real = dlsym(RTLD_NEXT, "utime");
    { FIX(path); return real(wp_p, t); }
}

int utimes(const char *path, const struct timeval t[2])
{
    static int (*real)(const char *, const struct timeval[2]);
    if (!real) real = dlsym(RTLD_NEXT, "utimes");
    { FIX(path); return real(wp_p, t); }
}

int rename(const char *a, const char *b)
{
    static int (*real)(const char *, const char *);
    char ba[WP_MAX], bb[WP_MAX];
    const char *pa, *pb;
    if (!real) real = dlsym(RTLD_NEXT, "rename");
    pa = wp_fix(a, ba, sizeof(ba));
    pb = wp_fix(b, bb, sizeof(bb));
    return real(pa, pb);
}

ssize_t readlink(const char *path, char *b, size_t n)
{
    static ssize_t (*real)(const char *, char *, size_t);
    if (!real) real = dlsym(RTLD_NEXT, "readlink");
    { FIX(path); return real(wp_p, b, n); }
}

char *realpath(const char *path, char *out)
{
    static char *(*real)(const char *, char *);
    if (!real) real = dlsym(RTLD_NEXT, "realpath");
    { FIX(path); return real(wp_p, out); }
}


/* ------------------------------------------------------------------
 * cmd.exe -> sh command-line quoting.
 *
 * The generator builds command lines the way cmd.exe reads them: inside a
 * double-quoted run, a backslash is an ordinary character, so a path ending
 * in a separator is written  "...\dir\"  .  /bin/sh reads the same text as an
 * escaped quote.  When perl hands /bin/sh a -c string we re-quote it: each
 * double-quoted run becomes a single-quoted sh word with the same literal
 * text, each bare word that contains a backslash is single-quoted, and
 * everything else (operators, redirections, spaces) is copied through.
 * No word is added, removed or reordered.
 */

static int wp_needs_quote(const char *w, size_t n)
{
    size_t i;
    for (i = 0; i < n; i++)
        if (w[i] == '\\')
            return 1;
    return 0;
}

static void wp_sq(const char *s, size_t n, char *out, size_t *o, size_t osz)
{
    size_t i;
    if (*o + 1 < osz) out[(*o)++] = '\'';
    for (i = 0; i < n; i++) {
        if (s[i] == '\'') {
            const char *esc = "'\\''";
            size_t k;
            for (k = 0; k < 4 && *o + 1 < osz; k++) out[(*o)++] = esc[k];
        } else if (*o + 1 < osz) {
            out[(*o)++] = s[i];
        }
    }
    if (*o + 1 < osz) out[(*o)++] = '\'';
    out[*o] = '\0';
}

static char *wp_cmd2sh(const char *cmd)
{
    size_t len = strlen(cmd);
    size_t osz = len * 4 + 16;
    char *out = malloc(osz);
    size_t o = 0, i = 0;
    int changed = 0;

    if (!out)
        return NULL;
    out[0] = '\0';
    while (i < len) {
        unsigned char c = (unsigned char)cmd[i];
        if (c == ' ' || c == '\t') {
            if (o + 1 < osz) out[o++] = (char)c;
            i++;
            continue;
        }
        /* one word: runs of unquoted text and double-quoted text */
        {
            size_t ws = i;
            int hasq = 0;
            while (i < len) {
                c = (unsigned char)cmd[i];
                if (c == ' ' || c == '\t')
                    break;
                if (c == '"') {
                    hasq = 1;
                    i++;
                    while (i < len && cmd[i] != '"')
                        i++;
                    if (i < len)
                        i++; /* closing quote */
                    continue;
                }
                i++;
            }
            {
                size_t wl = i - ws;
                const char *w = cmd + ws;
                if (!hasq && !wp_needs_quote(w, wl)) {
                    size_t k;
                    for (k = 0; k < wl && o + 1 < osz; k++) out[o++] = w[k];
                    out[o] = '\0';
                } else {
                    /* strip cmd.exe double quotes, then single-quote for sh */
                    char *lit = malloc(wl + 1);
                    size_t ll = 0, k;
                    if (!lit) { free(out); return NULL; }
                    for (k = 0; k < wl; k++)
                        if (w[k] != '"')
                            lit[ll++] = w[k];
                    lit[ll] = '\0';
                    wp_sq(lit, ll, out, &o, osz);
                    free(lit);
                    changed = 1;
                }
            }
        }
    }
    out[o] = '\0';
    if (!changed) {
        free(out);
        return NULL;
    }
    return out;
}

static char *const *wp_fix_argv(const char *path, char *const argv[], char **slot)
{
    const char *base;
    if (!path || !argv || !argv[0] || !argv[1] || !argv[2] || argv[3])
        return argv;
    base = strrchr(path, '/');
    base = base ? base + 1 : path;
    if (strcmp(base, "sh") && strcmp(base, "bash") && strcmp(base, "dash"))
        return argv;
    if (strcmp(argv[1], "-c"))
        return argv;
    *slot = wp_cmd2sh(argv[2]);
    {
        const char *log = getenv("WINPATH_LOG");
        if (log) {
            FILE *f = fopen(log, "a");
            if (f) {
                fprintf(f, "IN : %s\nOUT: %s\n\n", argv[2],
                        *slot ? *slot : "(unchanged)");
                fclose(f);
            }
        }
    }
    if (!*slot)
        return argv;
    {
        static __thread char *nv[4];
        nv[0] = argv[0];
        nv[1] = argv[1];
        nv[2] = *slot;
        nv[3] = NULL;
        return nv;
    }
}

int execv(const char *path, char *const argv[])
{
    static int (*real)(const char *, char *const[]);
    char *slot = NULL;
    if (!real) real = dlsym(RTLD_NEXT, "execv");
    { FIX(path); return real(wp_p, wp_fix_argv(wp_p, argv, &slot)); }
}

int execve(const char *path, char *const argv[], char *const envp[])
{
    static int (*real)(const char *, char *const[], char *const[]);
    char *slot = NULL;
    if (!real) real = dlsym(RTLD_NEXT, "execve");
    { FIX(path); return real(wp_p, wp_fix_argv(wp_p, argv, &slot), envp); }
}

int execvp(const char *path, char *const argv[])
{
    static int (*real)(const char *, char *const[]);
    char *slot = NULL;
    if (!real) real = dlsym(RTLD_NEXT, "execvp");
    { FIX(path); return real(wp_p, wp_fix_argv(wp_p, argv, &slot)); }
}

/* glibc's execl()/execlp() reach the kernel through an internal call that
 * never goes through the PLT, so they need their own wrappers. */
static int wp_collect(const char *arg0, va_list ap, char **out, int max)
{
    int n = 0;
    out[n++] = (char *)arg0;
    while (n < max - 1) {
        char *a = va_arg(ap, char *);
        out[n++] = a;
        if (!a)
            break;
    }
    out[n] = NULL;
    return n;
}

int execl(const char *path, const char *arg0, ...)
{
    char *args[256];
    va_list ap;
    va_start(ap, arg0);
    wp_collect(arg0, ap, args, 256);
    va_end(ap);
    return execv(path, args);
}

int execlp(const char *file, const char *arg0, ...)
{
    char *args[256];
    va_list ap;
    va_start(ap, arg0);
    wp_collect(arg0, ap, args, 256);
    va_end(ap);
    return execvp(file, args);
}

/* GNU make 4.3+ launches recipes with posix_spawn, which never reaches the
 * exec* wrappers above. */
#include <spawn.h>

int posix_spawn(pid_t *pid, const char *path,
                const posix_spawn_file_actions_t *fa,
                const posix_spawnattr_t *at, char *const argv[],
                char *const envp[])
{
    static int (*real)(pid_t *, const char *,
                       const posix_spawn_file_actions_t *,
                       const posix_spawnattr_t *, char *const[], char *const[]);
    char *slot = NULL;
    if (!real) real = dlsym(RTLD_NEXT, "posix_spawn");
    { FIX(path); return real(pid, wp_p, fa, at,
                             wp_fix_argv(wp_p, argv, &slot), envp); }
}

int posix_spawnp(pid_t *pid, const char *file,
                 const posix_spawn_file_actions_t *fa,
                 const posix_spawnattr_t *at, char *const argv[],
                 char *const envp[])
{
    static int (*real)(pid_t *, const char *,
                       const posix_spawn_file_actions_t *,
                       const posix_spawnattr_t *, char *const[], char *const[]);
    char *slot = NULL;
    if (!real) real = dlsym(RTLD_NEXT, "posix_spawnp");
    { FIX(file); return real(pid, wp_p, fa, at,
                             wp_fix_argv(wp_p, argv, &slot), envp); }
}
