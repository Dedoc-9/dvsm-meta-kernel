#ifndef DVSM_CORE_H
#define DVSM_CORE_H

#ifdef __cplusplus
extern "C" {
#endif

//! ============================================================
//! DVSM-π+++ / DQSDv2
//! Stable C ABI Surface
//! Author: Daniel J. Dillberg
//! Contact: BigDilly95@gmail.com
//! ============================================================

// ------------------------------------------------------------
// Ghost Space
// ------------------------------------------------------------

#define DVSM_NOMINAL   0
#define DVSM_COLLAPSE  1
#define DVSM_DIFFUSE   2
#define DVSM_ECHO      3
#define DVSM_BURST     4
#define DVSM_TRAP      5
#define DVSM_VACUUM    6

// ------------------------------------------------------------
// Opaque Handle
// ------------------------------------------------------------

typedef struct DVSM_Handle DVSM_Handle;

// ------------------------------------------------------------
// Parameters
// ------------------------------------------------------------

typedef struct DVSM_Params {
float dt;
float alpha;
float lambda;
float u_max;
unsigned int r;
} DVSM_Params;

// ------------------------------------------------------------
// Trace Frame
// ------------------------------------------------------------

typedef struct DVSM_TraceFrame {
unsigned long long frame;

```
float stress;
float novelty;
float drift;
float entropy;
float energy;

unsigned char ghost;
unsigned char contained;
```

} DVSM_TraceFrame;

// ------------------------------------------------------------
// ABI Exports
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

unsigned char dvsm_is_vacuum(
const DVSM_Handle* handle
);

void dvsm_free(
DVSM_Handle* handle
);

#ifdef __cplusplus
}
#endif

#endif // DVSM_CORE_H
