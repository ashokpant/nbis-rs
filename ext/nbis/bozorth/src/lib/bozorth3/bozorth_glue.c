#include <stdio.h>

/*  ---- globals required by bozorth3.c --------------------------- */
/*
 * Legacy NBIS code calls fprintf(errorfp, ...) unconditionally on some
 * paths (notably qq[] overflow in bz_match_score / bz_sift). Those paths
 * are NOT gated by NOVERBOSE. Leaving errorfp as NULL causes a segfault
 * during 1:N matching when overflow is hit — exactly the gallery-search
 * crash mode. Use a silent sink so logs stay quiet but fprintf is safe.
 */
FILE *errorfp = NULL;

int  m1_xyt  = 0;              /* 0 = CW angle math (matches NBIS default)      */
int  min_computable_minutiae = 15;  /* same hard-coded default NBIS uses       */

static void nbis_bozorth_ensure_errorfp(void)
{
	if (errorfp != NULL)
		return;
#if defined(_WIN32)
	errorfp = fopen("NUL", "w");
#else
	errorfp = fopen("/dev/null", "w");
#endif
	/* Last resort: never leave NULL (stderr is better than a segfault). */
	if (errorfp == NULL)
		errorfp = stderr;
}

#if defined(_MSC_VER)
#pragma section(".CRT$XCU", read)
static void __cdecl nbis_bozorth_init(void);
__declspec(allocate(".CRT$XCU")) void (__cdecl *nbis_bozorth_init_)(void) = nbis_bozorth_init;
static void __cdecl nbis_bozorth_init(void)
{
	nbis_bozorth_ensure_errorfp();
}
#else
__attribute__((constructor))
static void nbis_bozorth_init(void)
{
	nbis_bozorth_ensure_errorfp();
}
#endif
