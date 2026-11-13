/*
 * hashloader.hdll - runtime bridge for HashLoader.
 *
 * HashLink keeps the type registry of every .hl module isolated: a plugin
 * loaded with sys_load_plugin cannot see the game classes with
 * Type.resolveClass() or $allTypes(), and sys_resolve_type() always resolves
 * against the *boot* module.
 *
 * HashLoader boots its payload as the boot module, so the game is just
 * another plugin. This bridge finds the game module inside the live process
 * and exposes its class objects, so mods never have to carry native code of
 * their own.
 *
 * The module is located by scanning the process writable memory for valid
 * hl_code structures. Candidates are validated (version, counters, readable
 * type table) and scored by the classes they contain. The module that calls
 * init() is excluded, so a mod never resolves itself by mistake.
 */

#ifdef _WIN32
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <tlhelp32.h>
#else
#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif
#include <errno.h>
#include <fcntl.h>
#include <sys/uio.h>
#include <unistd.h>
#endif
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define HL_NAME(x) hashloader_##x
#include <hl.h>

typedef struct {
	int version;
	int nints;
	int nfloats;
	int nstrings;
	int nbytes;
	int ntypes;
	int nglobals;
	int nnatives;
	int nfunctions;
	int nconstants;
	int entrypoint;
	int ndebugfiles;
	bool hasdebug;
	int*		ints;
	double*		floats;
	char**		strings;
	int*		strings_lens;
	char*		bytes;
	int*		bytes_pos;
	char**		debugfiles;
	int*		debugfiles_lens;
	uchar**		ustrings;
	hl_type*	types;
	hl_type**	globals;
	void*		natives;
	void*		functions;
	void*		constants;
	hl_alloc	alloc;
	hl_alloc	falloc;
} hl_code;

typedef struct {
	hl_code *code;
	int codesize;
	int globals_size;
	int *globals_indexes;
	unsigned char *globals_data;
	void **functions_ptrs;
	int *functions_indexes;
	void *jit_code;
	void *hash;
	void *jit_debug;
	void *jit_ctx;
	hl_module_context ctx;
} hl_module;

#define HLB_VERSION 1
#define HLB_MAX_CALLBACKS 128

/* Fases de diagnostico. Se conservan en una global para poder consultarlas
 * desde el catch de Haxe si una lectura invalida aborta la llamada. */
enum {
	PH_NONE = 0,
	PH_ENTER = 1,
	PH_CALLER = 2,
	PH_SUSPEND = 3,
	PH_SCAN = 10,
	PH_SCAN_LOOK = 11,
	PH_SCAN_SCORE = 12,
	PH_SCAN_ACCEPT = 13,
	PH_SCAN_DONE = 14,
	PH_RESUME = 15,
	PH_OK = 16,
};

static int g_phase = PH_NONE;

static hl_code *g_game_code = NULL;
static hl_code *g_caller_code = NULL;
static hl_code *g_fallback_code = NULL;
static int g_fallback_types = -1;
static int g_init_calls = 0;

static vdynamic *g_context = NULL;

static vdynamic *g_pre_cbs[HLB_MAX_CALLBACKS];
static int g_pre_count = 0;
static vdynamic *g_frame_cbs[HLB_MAX_CALLBACKS];
static int g_frame_count = 0;

/* ------------------------------------------------------------------ memory */

#define MEM_CACHE_SLOTS 16

typedef struct {
	uintptr_t base;
	uintptr_t end;
} mem_range;

static mem_range g_ok_cache[MEM_CACHE_SLOTS];
static int g_ok_count = 0;
static int g_ok_next = 0;
static uintptr_t g_last_bad_base = 0;
static uintptr_t g_last_bad_end = 0;

#ifndef _WIN32
typedef struct {
	uintptr_t base;
	uintptr_t end;
	int rw_priv;
} linux_map_region;

static linux_map_region *g_maps = NULL;
static int g_maps_count = 0;
static int g_maps_cap = 0;
static int g_maps_valid = 0;
static uintptr_t g_maps_max_end = 0;
static int g_self_mem_fd = -1;

static int is_skippable_mapping_path(const char *path) {
	if (!path || !*path) return 0;
	if (strcmp(path, "[vvar]") == 0 ||
		strcmp(path, "[vvar_vclock]") == 0 ||
		strcmp(path, "[vsyscall]") == 0 ||
		strcmp(path, "[vdso]") == 0)
		return 1;
	if (strncmp(path, "/dev/", 5) == 0) return 1;
	return 0;
}

static void linux_refresh_maps(void) {
	FILE *fp;
	char line[512];

	g_maps_count = 0;
	g_maps_max_end = 0;
	g_maps_valid = 1;
	fp = fopen("/proc/self/maps", "r");
	if (!fp) return;

	while (fgets(line, sizeof(line), fp)) {
		unsigned long long s = 0, e = 0;
		char perms[8] = {0};
		int n = 0;
		char *p;
		const char *path = "";
		int tok;

		if (sscanf(line, "%llx-%llx %7s %n", &s, &e, perms, &n) != 3)
			continue;
		if (perms[0] != 'r') continue;
		if (e <= s) continue;

		/* Saltar offset, dev e inode hasta llegar al pathname opcional */
		p = line + n;
		for (tok = 0; tok < 3; tok++) {
			while (*p == ' ' || *p == '\t') p++;
			while (*p && *p != ' ' && *p != '\t' && *p != '\n') p++;
		}
		while (*p == ' ' || *p == '\t') p++;
		if (*p && *p != '\n') {
			size_t len = strlen(p);
			while (len > 0 && (p[len - 1] == '\n' || p[len - 1] == '\r'))
				p[--len] = '\0';
			path = p;
		}
		if (is_skippable_mapping_path(path)) continue;

		if (g_maps_count == g_maps_cap) {
			int ncap = g_maps_cap ? g_maps_cap * 2 : 128;
			linux_map_region *nr = (linux_map_region *)realloc(
				g_maps, (size_t)ncap * sizeof(linux_map_region));
			if (!nr) break;
			g_maps = nr;
			g_maps_cap = ncap;
		}
		g_maps[g_maps_count].base = (uintptr_t)s;
		g_maps[g_maps_count].end = (uintptr_t)e;
		g_maps[g_maps_count].rw_priv = (perms[1] == 'w' && perms[3] == 'p') ? 1 : 0;
		if ((uintptr_t)e > g_maps_max_end) g_maps_max_end = (uintptr_t)e;
		g_maps_count++;
	}
	fclose(fp);
}

static void linux_ensure_maps(void) {
	if (!g_maps_valid)
		linux_refresh_maps();
}

static int linux_find_region(uintptr_t addr, uintptr_t *out_base, uintptr_t *out_end) {
	int lo, hi;
	linux_ensure_maps();
	lo = 0;
	hi = g_maps_count - 1;
	while (lo <= hi) {
		int mid = lo + (hi - lo) / 2;
		if (addr < g_maps[mid].base) {
			hi = mid - 1;
		} else if (addr >= g_maps[mid].end) {
			lo = mid + 1;
		} else {
			if (out_base) *out_base = g_maps[mid].base;
			if (out_end) *out_end = g_maps[mid].end;
			return 1;
		}
	}
	return 0;
}

static int safe_read_bytes(uintptr_t addr, void *dst, size_t len) {
	pid_t pid = getpid();
	struct iovec local_iov;
	struct iovec remote_iov;
	ssize_t n;

	if (!addr || !dst || len == 0) return 0;
	local_iov.iov_base = dst;
	local_iov.iov_len = len;
	remote_iov.iov_base = (void *)addr;
	remote_iov.iov_len = len;

	n = process_vm_readv(pid, &local_iov, 1, &remote_iov, 1, 0);
	if (n == (ssize_t)len) return 1;
	if (n >= 0 || (errno != ENOSYS && errno != EPERM)) return 0;

	if (g_self_mem_fd < 0)
		g_self_mem_fd = open("/proc/self/mem", O_RDONLY);
	if (g_self_mem_fd < 0) return 0;
	if (addr > (uintptr_t)0x7fffffffffffffffULL) return 0;
	n = pread(g_self_mem_fd, dst, len, (off_t)addr);
	return n == (ssize_t)len;
}
#endif

static void mem_cache_reset(void) {
	g_ok_count = 0;
	g_ok_next = 0;
	g_last_bad_base = 0;
	g_last_bad_end = 0;
#ifndef _WIN32
	g_maps_valid = 0;
#endif
}

/* Comprobacion autoritativa de que un rango es legible. Cacheamos las ultimas
 * regiones consultadas dentro de la llamada nativa actual para evitar miles de
 * syscalls VirtualQuery al alternar entre distintos bloques de heap (types,
 * obj, ustrings). */
static int mem_readable(const void *p, size_t len) {
	uintptr_t a = (uintptr_t)p;
	uintptr_t end;
	int i;
	if (!p || len == 0) return 0;
	if (a + len < a) return 0;
	end = a + len;
	for (i = 0; i < g_ok_count; i++) {
		if (a >= g_ok_cache[i].base && end <= g_ok_cache[i].end)
			return 1;
	}
	if (a >= g_last_bad_base && end <= g_last_bad_end) return 0;
	while (a < end) {
		uintptr_t rb = 0, re = 0;
#ifdef _WIN32
		MEMORY_BASIC_INFORMATION mbi;
		if (VirtualQuery((LPCVOID)a, &mbi, sizeof(mbi)) != sizeof(mbi))
			return 0;
		rb = (uintptr_t)mbi.BaseAddress;
		re = rb + mbi.RegionSize;
		if (mbi.State != MEM_COMMIT || (mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD))) {
			if (rb < re) {
				g_last_bad_base = rb;
				g_last_bad_end = re;
			}
			return 0;
		}
#else
		if (!linux_find_region(a, &rb, &re))
			return 0;
#endif
		if (re <= a) return 0;
		g_ok_cache[g_ok_next].base = rb;
		g_ok_cache[g_ok_next].end = re;
		g_ok_next = (g_ok_next + 1) % MEM_CACHE_SLOTS;
		if (g_ok_count < MEM_CACHE_SLOTS) g_ok_count++;
		if (re > end) re = end;
		a = re;
	}
	return 1;
}

/* ------------------------------------------------------------------- types */

/* HashLink stores type/field names as UTF-16 (hl_get_ustring converts the
 * UTF-8 names of the bytecode at load time), and Haxe strings keep their
 * characters as UTF-16 too. Compare both buffers as UTF-16 code units. */
static int u16_eq(const uchar *a, const uchar *b) {
	int i;
	if (!a || !b) return 0;
	for (i = 0; i < 512; i++) {
		if (!mem_readable(a + i, sizeof(uchar))) return 0;
		if (!mem_readable(b + i, sizeof(uchar))) return 0;
		if (a[i] != b[i]) return 0;
		if (a[i] == 0) return 1;
	}
	return 0;
}

/* Compare a UTF-16 type name against an ASCII literal. */
static int u16_eq_ascii(const uchar *u, const char *ascii) {
	int i;
	if (!u) return 0;
	for (i = 0; ; i++) {
		unsigned char c = (unsigned char)ascii[i];
		if (!mem_readable(u + i, sizeof(uchar))) return 0;
		if (u[i] != (uchar)c) return 0;
		if (c == 0) return 1;
	}
}

static int u16_eq_fast(const uchar *a, const uchar *b) {
	int i;
	if (!a || !b) return 0;
	for (i = 0; i < 512; i++) {
		if (a[i] != b[i]) return 0;
		if (a[i] == 0) return 1;
	}
	return 0;
}

static const uchar *type_name(hl_type *t) {
	hl_type_obj *o;
	if (!t) return NULL;
	if (t->kind != HOBJ && t->kind != HSTRUCT) return NULL;
	o = t->obj;
	if (!o) return NULL;
	if (!mem_readable(o, sizeof(hl_type_obj))) return NULL;
	if (!o->name) return NULL;
	if (!mem_readable(o->name, sizeof(uchar))) return NULL;
	return o->name;
}

static const uchar *type_name_fast(hl_type *t) {
	hl_type_obj *o;
	if (!t) return NULL;
	if (t->kind != HOBJ && t->kind != HSTRUCT) return NULL;
	o = t->obj;
	if (!o) return NULL;
	return o->name;
}

static int code_looks_valid(hl_code *c) {
	int check_n;
	int i;
	if (!c) return 0;
	if (!mem_readable(c, sizeof(hl_code))) return 0;
	if (c->version < 2 || c->version > 6) return 0;
	if (c->nints < 0 || c->nfloats < 0 || c->nstrings < 0 || c->nbytes < 0) return 0;
	if (c->nnatives < 0 || c->nnatives > 5000000 || c->nconstants < 0) return 0;
	if (c->ntypes <= 0 || c->ntypes > 500000) return 0;
	if (c->nfunctions <= 0 || c->nfunctions > 5000000) return 0;
	if (c->nglobals < 0 || c->nglobals > 5000000) return 0;
	if (c->entrypoint < 0 || c->entrypoint >= (c->nfunctions + c->nnatives)) return 0;
	if (!c->types || !c->functions) return 0;
	if (c->nstrings > 0 && (!c->strings || !mem_readable(c->strings, sizeof(char *)))) return 0;
	if (!mem_readable(c->types, (size_t)c->ntypes * sizeof(hl_type))) return 0;
	if (!mem_readable(c->functions, sizeof(void *))) return 0;
	check_n = c->ntypes < 16 ? c->ntypes : 16;
	for (i = 0; i < check_n; i++) {
		unsigned int k = (unsigned int)c->types[i].kind;
		if (k >= (unsigned int)HLAST) return 0;
	}
	return 1;
}

static int code_score(hl_code *c);

static int code_score(hl_code *c) {
	int score = 0;
	int i;
	for (i = 0; i < c->ntypes; i++) {
		const uchar *nm = type_name(&c->types[i]);
		uchar c0;
		if (!nm) continue;
		c0 = nm[0];
		if (c0 == 'h') {
			if (u16_eq_ascii(nm, "hxd.System")) score += 1000;
			else if (u16_eq_ascii(nm, "hxd.App")) score += 100;
			else if (u16_eq_ascii(nm, "h2d.Scene")) score += 50;
		} else if (c0 == 'l') {
			if (u16_eq_ascii(nm, "lime.app.Application")) score += 1000;
		} else if (c0 == 'p') {
			if (u16_eq_ascii(nm, "peote.view.PeoteView")) score += 200;
		} else if (c0 == 'f') {
			if (u16_eq_ascii(nm, "flixel.FlxG")) score += 200;
		} else if (c0 == 'G') {
			if (u16_eq_ascii(nm, "GameState")) score += 100;
		} else if (c0 == 's') {
			if (u16_eq_ascii(nm, "sdl.Window")) score += 50;
		} else if (c0 == 'M') {
			if (u16_eq_ascii(nm, "Main")) score += 20;
		} else if (c0 == 'H' || c0 == 'L') {
			if (u16_eq_ascii(nm, "HashLoader") || u16_eq_ascii(nm, "LoaderPayload")) score -= 500;
		}
	}
	return score;
}

/* The caller module is derived from the type passed by Haxe. hl_type_obj->m
 * points to the hl_module_context embedded in its hl_module, so subtracting
 * the field offset gives us the module and its code. This lets the scan skip
 * the module that is asking. */
static hl_code *caller_code_from_type(hl_type *t) {
	hl_module_context *ctx;
	hl_module *m;
	if (!t || t->kind != HOBJ || !t->obj) return NULL;
	if (!mem_readable(t->obj, sizeof(hl_type_obj))) return NULL;
	ctx = t->obj->m;
	if (!ctx) return NULL;
	if (!mem_readable(ctx, sizeof(hl_module_context))) return NULL;
	m = (hl_module *)((char *)ctx - offsetof(hl_module, ctx));
	if (!mem_readable(m, sizeof(hl_module))) return NULL;
	if (!code_looks_valid(m->code)) return NULL;
	return m->code;
}

static void consider_candidate(
	hl_code *c,
	hl_code **best,
	int *best_score,
	int *best_types
) {
	int score;
	g_phase = PH_SCAN_LOOK;
	if (!code_looks_valid(c)) return;
	g_phase = PH_SCAN_SCORE;
	score = code_score(c);
	/* Solo aceptamos modulos que contengan clases conocidas del
	 * juego. Cualquier bloque de heap puede parecerse a un
	 * hl_code, y un falso positivo cacheado impediria encontrar
	 * el modulo real mas tarde. */
	if (score > 0) {
		g_phase = PH_SCAN_ACCEPT;
		if (score > *best_score || (score == *best_score && c->ntypes > *best_types)) {
			*best = c;
			*best_score = score;
			*best_types = c->ntypes;
		}
	}
	if (score >= 0 && c->ntypes >= 16 && c->ntypes > g_fallback_types) {
		g_fallback_code = c;
		g_fallback_types = c->ntypes;
	}
}

#ifdef _WIN32
static int is_private_writable(DWORD type, DWORD protect) {
	DWORD p = protect & 0xff;
	if (type != MEM_PRIVATE) return 0;
	return p == PAGE_READWRITE || p == PAGE_WRITECOPY ||
		p == PAGE_EXECUTE_READWRITE || p == PAGE_EXECUTE_WRITECOPY;
}

/* Escanear la memoria del proceso mientras otro hilo (el que carga main.hl)
 * sigue asignando y liberando memoria puede dejarnos leyendo paginas ya
 * liberadas. HashLink convierte esas violaciones en excepciones Haxe, asi que
 * no se pueden capturar con __try/__except. Congelamos el resto de hilos
 * durante el escaneo: solo leemos memoria, no llamamos a HashLink. */
static int suspend_other_threads(HANDLE **out) {
	DWORD pid = GetCurrentProcessId();
	DWORD self = GetCurrentThreadId();
	HANDLE snap;
	THREADENTRY32 te;
	DWORD *tids = NULL;
	HANDLE *list = NULL;
	int ntids = 0;
	int cap = 0;
	int count = 0;
	int i;

	*out = NULL;

	/* Primera pasada: recoger IDs. Toda la memoria se reserva antes de
	 * suspender nada, para no bloquearnos en el heap si otro hilo lo tiene. */
	snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
	if (snap == INVALID_HANDLE_VALUE) return 0;

	te.dwSize = sizeof(te);
	if (Thread32First(snap, &te)) {
		do {
			if (te.th32OwnerProcessID != pid || te.th32ThreadID == self) continue;
			if (ntids == cap) {
				int ncap = cap ? cap * 2 : 16;
				DWORD *nl = (DWORD *)realloc(tids, sizeof(DWORD) * ncap);
				if (!nl) break;
				tids = nl;
				cap = ncap;
			}
			tids[ntids++] = te.th32ThreadID;
		} while (Thread32Next(snap, &te));
	}
	CloseHandle(snap);
	if (ntids == 0) {
		free(tids);
		return 0;
	}

	list = (HANDLE *)malloc(sizeof(HANDLE) * ntids);
	if (!list) {
		free(tids);
		return 0;
	}

	for (i = 0; i < ntids; i++) {
		HANDLE h = OpenThread(THREAD_SUSPEND_RESUME, FALSE, tids[i]);
		if (!h) continue;
		if (SuspendThread(h) == (DWORD)-1) {
			CloseHandle(h);
			continue;
		}
		list[count++] = h;
	}
	free(tids);
	*out = list;
	return count;
}

static void resume_other_threads(HANDLE *list, int count) {
	int i;
	if (!list) return;
	for (i = count - 1; i >= 0; i--) {
		ResumeThread(list[i]);
		CloseHandle(list[i]);
	}
	free(list);
}

static hl_code *scan_for_game_code(hl_code *exclude) {
	SYSTEM_INFO si;
	uintptr_t addr;
	uintptr_t maxaddr;
	uintptr_t minaddr;
	hl_code *best = NULL;
	int best_score = -1000000;
	int best_types = -1;

	g_fallback_code = NULL;
	g_fallback_types = -1;

	GetSystemInfo(&si);
	addr = (uintptr_t)si.lpMinimumApplicationAddress;
	maxaddr = (uintptr_t)si.lpMaximumApplicationAddress;
	minaddr = addr;

	g_phase = PH_SCAN;
	while (addr < maxaddr) {
		MEMORY_BASIC_INFORMATION mbi;
		uintptr_t base, end, p;
		if (VirtualQuery((LPCVOID)addr, &mbi, sizeof(mbi)) != sizeof(mbi))
			break;
		base = (uintptr_t)mbi.BaseAddress;
		end = base + mbi.RegionSize;
		if (mbi.State == MEM_COMMIT && is_private_writable(mbi.Type, mbi.Protect) &&
			!(mbi.Protect & PAGE_GUARD)) {
			p = (base + 7) & ~(uintptr_t)7;
			for (; p + sizeof(uintptr_t) <= end; p += sizeof(uintptr_t)) {
				uintptr_t v;
				hl_code *c;
				v = *(volatile uintptr_t *)p;
				if (v < minaddr || v >= maxaddr) continue;
				if (v & 7) continue;
				if (!mem_readable((void *)v, sizeof(hl_code))) continue;
				c = (hl_code *)v;
				if (c == exclude) continue;
				if (c == g_game_code) continue;
				if (c == best || c == g_fallback_code) continue;
				consider_candidate(c, &best, &best_score, &best_types);
			}
		}
		if (end <= addr) break;
		addr = end;
	}
	g_phase = PH_SCAN_DONE;
	return best;
}
#else
static void evaluate_candidate_ptr(
	uintptr_t v,
	hl_code *exclude,
	hl_code **best,
	int *best_score,
	int *best_types
) {
	hl_code probe_c;
	hl_code *c;

	if (v < (uintptr_t)0x10000 || v >= g_maps_max_end) return;
	if (v & (sizeof(uintptr_t) - 1)) return;
	c = (hl_code *)v;
	if (c == exclude || c == g_game_code || c == *best || c == g_fallback_code) return;
	if (!mem_readable(c, sizeof(hl_code))) return;
	if (!safe_read_bytes(v, &probe_c, sizeof(hl_code))) return;
	if (probe_c.version < 2 || probe_c.version > 6) return;
	if (probe_c.ntypes <= 0 || probe_c.ntypes > 500000) return;
	if (probe_c.nnatives < 0 || probe_c.nnatives > 5000000) return;
	if (probe_c.nfunctions <= 0 || probe_c.nfunctions > 5000000) return;
	if (probe_c.nglobals < 0 || probe_c.nglobals > 5000000) return;
	if (probe_c.entrypoint < 0 || probe_c.entrypoint >= (probe_c.nfunctions + probe_c.nnatives)) return;
	if (!probe_c.types || !probe_c.functions) return;
	if (!mem_readable(probe_c.types, (size_t)probe_c.ntypes * sizeof(hl_type))) return;

	consider_candidate(c, best, best_score, best_types);
}

static hl_code *scan_for_game_code(hl_code *exclude) {
	enum { CHUNK_SIZE = 65536, PAGE_STEP = 4096 };
	static unsigned char buf[CHUNK_SIZE];
	hl_code *best = NULL;
	int best_score = -1000000;
	int best_types = -1;
	int ri;

	g_fallback_code = NULL;
	g_fallback_types = -1;

	linux_ensure_maps();
	g_phase = PH_SCAN;

	for (ri = 0; ri < g_maps_count; ri++) {
		uintptr_t addr, end;
		if (!g_maps[ri].rw_priv) continue;
		addr = (g_maps[ri].base + (sizeof(uintptr_t) - 1)) & ~(uintptr_t)(sizeof(uintptr_t) - 1);
		end = g_maps[ri].end;

		while (addr + sizeof(uintptr_t) <= end) {
			size_t avail = (size_t)(end - addr);
			size_t chunk = avail > CHUNK_SIZE ? CHUNK_SIZE : avail;
			size_t off;
			/* Evita cruzar limites de nuestro propio buffer o regiones dinamicas */
			if (safe_read_bytes(addr, buf, chunk)) {
				for (off = 0; off + sizeof(uintptr_t) <= chunk; off += sizeof(uintptr_t)) {
					uintptr_t v;
					memcpy(&v, buf + off, sizeof(uintptr_t));
					evaluate_candidate_ptr(v, exclude, &best, &best_score, &best_types);
				}
				addr += chunk;
			} else {
				/* Si falla una pagina dentro del bloque, probamos pagina a pagina sin abortar */
				size_t sub = chunk > PAGE_STEP ? PAGE_STEP : chunk;
				if (safe_read_bytes(addr, buf, sub)) {
					for (off = 0; off + sizeof(uintptr_t) <= sub; off += sizeof(uintptr_t)) {
						uintptr_t v;
						memcpy(&v, buf + off, sizeof(uintptr_t));
						evaluate_candidate_ptr(v, exclude, &best, &best_score, &best_types);
					}
				}
				addr += sub;
			}
		}
	}
	g_phase = PH_SCAN_DONE;
	return best;
}
#endif

static int type_by_name(const uchar *want, hl_type **out) {
	hl_code *c = g_game_code;
	int i;
	uchar w0;
	if (!c || !want) return 0;
	w0 = want[0];
	for (i = 0; i < c->ntypes; i++) {
		hl_type *t = &c->types[i];
		const uchar *nm = type_name_fast(t);
		if (!nm || nm[0] != w0 || !u16_eq_fast(nm, want)) continue;
		if (out) *out = t;
		return 1;
	}
	return 0;
}

/* ----------------------------------------------------------------- dispatch */

static void run_callbacks(vdynamic **list, int count) {
	int i;
	for (i = 0; i < count; i++) {
		vclosure *cl = (vclosure *)list[i];
		bool is_exc = false;
		if (!cl) continue;
		hl_dyn_call_safe(cl, NULL, 0, &is_exc);
		if (is_exc)
			fprintf(stderr, "[HashLoader] callback Haxe lanzo una excepcion\n");
	}
}

static void add_callback(vdynamic **list, int *count, vdynamic *cb) {
	if (!cb || *count >= HLB_MAX_CALLBACKS) return;
	list[*count] = cb;
	hl_add_root(&list[*count]);
	(*count)++;
}

/* ------------------------------------------------------------------- exports */

HL_PRIM int HL_NAME(available)(void) {
	return 1;
}

HL_PRIM int HL_NAME(version)(void) {
	return HLB_VERSION;
}

HL_PRIM int HL_NAME(phase)(void) {
	return g_phase;
}

HL_PRIM int HL_NAME(init)(hl_type *caller) {
	hl_code *found;
#ifdef _WIN32
	HANDLE *handles = NULL;
	int nhandles;
#endif

	mem_cache_reset();
	g_phase = PH_ENTER;
	if (g_game_code && code_looks_valid(g_game_code)) {
		g_phase = PH_OK;
		return g_game_code->ntypes;
	}

	g_init_calls++;
	g_phase = PH_CALLER;
	g_caller_code = caller_code_from_type(caller);

	g_phase = PH_SUSPEND;
#ifdef _WIN32
	nhandles = suspend_other_threads(&handles);
	mem_cache_reset();
	found = scan_for_game_code(g_caller_code);
	g_phase = PH_RESUME;
	resume_other_threads(handles, nhandles);
#else
	mem_cache_reset();
	found = scan_for_game_code(g_caller_code);
	g_phase = PH_RESUME;
#endif
	/* Si aun no hay un modulo con clases reconocibles, seguimos esperando:
	 * el juego puede estar cargandose. Tras unos segundos aceptamos el
	 * modulo mas grande como ultimo recurso (juegos no basados en Heaps). */
	if (!found && g_caller_code != NULL && g_init_calls > 240)
		found = g_fallback_code;
	if (!found) {
		g_phase = PH_NONE;
		return 0;
	}
	g_game_code = found;
	g_phase = PH_OK;
	return g_game_code->ntypes;
}

HL_PRIM int HL_NAME(type_count)(void) {
	return g_game_code ? g_game_code->ntypes : -1;
}

HL_PRIM int HL_NAME(has_type)(vbyte *name) {
	hl_type *t = NULL;
	if (!g_game_code || !name) return 0;
	mem_cache_reset();
	return type_by_name((const uchar *)name, &t);
}

HL_PRIM vdynamic *HL_NAME(resolve)(vbyte *name) {
	hl_type *t = NULL;
	if (!g_game_code || !name) return NULL;
	mem_cache_reset();
	if (!type_by_name((const uchar *)name, &t)) return NULL;
	if (t->kind != HOBJ && t->kind != HSTRUCT) return NULL;
	if (!t->obj || !t->obj->global_value) return NULL;
	if (!mem_readable(t->obj->global_value, sizeof(void *))) return NULL;
	return *t->obj->global_value;
}

HL_PRIM int HL_NAME(type_index)(vbyte *name) {
	hl_code *c = g_game_code;
	int i;
	if (!c || !name) return -1;
	mem_cache_reset();
	if (!mem_readable(c, sizeof(hl_code))) return -1;
	if (!c->types || !mem_readable(c->types, (size_t)c->ntypes * sizeof(hl_type)))
		return -1;
	for (i = 0; i < c->ntypes; i++) {
		const uchar *nm = type_name(&c->types[i]);
		if (nm && u16_eq(nm, (const uchar *)name)) return i;
	}
	return -1;
}

HL_PRIM void *HL_NAME(code_ptr)(void) {
	return g_game_code;
}

HL_PRIM int HL_NAME(excluded_caller)(void) {
	return g_caller_code != NULL && g_caller_code != g_game_code;
}

/* --- game context (the Main / App instance, set by the loader payload) --- */

HL_PRIM void HL_NAME(set_context)(vdynamic *v) {
	if (g_context) {
		hl_remove_root(&g_context);
		g_context = NULL;
	}
	if (v) {
		g_context = v;
		hl_add_root(&g_context);
	}
}

HL_PRIM vdynamic *HL_NAME(get_context)(void) {
	return g_context;
}

/* --- frame callbacks ------------------------------------------------------ */

HL_PRIM void HL_NAME(add_pre_cb)(vdynamic *cb) {
	add_callback(g_pre_cbs, &g_pre_count, cb);
}

HL_PRIM void HL_NAME(add_frame_cb)(vdynamic *cb) {
	add_callback(g_frame_cbs, &g_frame_count, cb);
}

HL_PRIM int HL_NAME(pre_cb_count)(void) {
	return g_pre_count;
}

HL_PRIM int HL_NAME(frame_cb_count)(void) {
	return g_frame_count;
}

HL_PRIM void HL_NAME(pre_tick)(void) {
	run_callbacks(g_pre_cbs, g_pre_count);
}

HL_PRIM void HL_NAME(frame_tick)(void) {
	run_callbacks(g_frame_cbs, g_frame_count);
}

/* ------------------------------------------------------------------- prims */

DEFINE_PRIM(_BOOL, available, _NO_ARG);
DEFINE_PRIM(_I32, version, _NO_ARG);
DEFINE_PRIM(_I32, phase, _NO_ARG);
DEFINE_PRIM(_I32, init, _TYPE);
DEFINE_PRIM(_I32, type_count, _NO_ARG);
DEFINE_PRIM(_BOOL, has_type, _BYTES);
DEFINE_PRIM(_DYN, resolve, _BYTES);
DEFINE_PRIM(_I32, type_index, _BYTES);
DEFINE_PRIM(_BOOL, excluded_caller, _NO_ARG);
DEFINE_PRIM(_VOID, set_context, _DYN);
DEFINE_PRIM(_DYN, get_context, _NO_ARG);
DEFINE_PRIM(_VOID, add_pre_cb, _FUN(_VOID, _NO_ARG));
DEFINE_PRIM(_VOID, add_frame_cb, _FUN(_VOID, _NO_ARG));
DEFINE_PRIM(_I32, pre_cb_count, _NO_ARG);
DEFINE_PRIM(_I32, frame_cb_count, _NO_ARG);
DEFINE_PRIM(_VOID, pre_tick, _NO_ARG);
DEFINE_PRIM(_VOID, frame_tick, _NO_ARG);
