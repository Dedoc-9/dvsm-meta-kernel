#ifndef DVSM_CORE_H
#define DVSM_CORE_H

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================
// DVSM-π+++ / DQSDv2 · STABLE C ABI SURFACE (FINAL)
// ============================================================
// PURPOSE:
// - Cross-language deterministic ABI (Rust / UE5 / C++)
// - No assumptions about Rust layout
// - No logic leakage
//
// Author: Daniel J. Dillberg
// ============================================================

#include <stdint.h>

// ------------------------------------------------------------
// GHOST SPACE (STATE MACHINE ENUM)
// ------------------------------------------------------------

#define DVSM_NOMINAL   0
#define DVSM_COLLAPSE  1
#define DVSM_DIFFUSE   2
#define DVSM_ECHO      3
#define DVSM_BURST     4
#define DVSM_TRAP      5
#define DVSM_VACUUM    6

// ------------------------------------------------------------
// OPAQUE HANDLE (RUST OWNED)
// ------------------------------------------------------------

typedef struct DVSM_Handle DVSM_Handle;

// ------------------------------------------------------------
// PARAMETERS (ABI STABLE)
// ------------------------------------------------------------

typedef struct DVSM_Params {
    float dt;
    float alpha;
    float lambda;
    float u_max;
    uint32_t r;
} DVSM_Params;

// ------------------------------------------------------------
// TRACE FRAME (FIXED LAYOUT)
// ------------------------------------------------------------

typedef struct DVSM_TraceFrame {
    uint64_t frame;

    float stress;
    float novelty;
    float drift;
    float entropy;
    float energy;

    uint8_t ghost;
    uint8_t contained;

    // padding for ABI alignment safety (future-proofing)
    uint8_t _pad[6];
} DVSM_TraceFrame;

// ------------------------------------------------------------
// ABI FUNCTIONS
// ------------------------------------------------------------

DVSM_Handle* dvsm_init(
    const DVSM_Params* params
);

int dvsm_step(
    DVSM_Handle* handle,
    const float* input,
    DVSM_TraceFrame* trace_out
);

int dvsm_recalibrate(
    DVSM_Handle* handle
);

uint8_t dvsm_is_vacuum(
    const DVSM_Handle* handle
);

void dvsm_free(
    DVSM_Handle* handle
);

#ifdef __cplusplus
}
#endif

#endif // DVSM_CORE_H
