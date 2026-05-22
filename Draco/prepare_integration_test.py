#!/usr/bin/env python3
"""
prepare_integration_test.py - Prepare Day 8 Tier 3 Integration Test Data

Downloads 1MBN.pdb from PDB database and runs pdb_to_torsion.py to generate
the 740-byte TorsionArray binary required for the Rust integration test.

USAGE:
    python prepare_integration_test.py

PREREQUISITES:
    - Python 3.x with urllib, sys
    - pdb_to_torsion.py in same directory
    - BioPython installed (pip install biopython)

OUTPUT:
    - 1MBN.pdb (downloaded, 46 KB)
    - test_torsion.bin (generated, 740 bytes)
    - Integration test ready to run: cargo test --test test_day8_integration
"""

import sys
import os
import subprocess
import urllib.request
from pathlib import Path

# ============================================================================
# CONFIGURATION
# ============================================================================

PDB_ID = "1MBN"  # Myoglobin
PDB_URL = f"https://files.rcsb.org/download/{PDB_ID}.pdb"
PDB_FILE = f"{PDB_ID}.pdb"
TEST_BINARY = "test_torsion.bin"
PARSER_SCRIPT = "pdb_to_torsion.py"

# ============================================================================
# DOWNLOAD PDB FILE
# ============================================================================

def download_pdb(pdb_id: str, url: str, output_file: str) -> bool:
    """Download PDB file from RCSB database."""
    if Path(output_file).exists():
        print(f"✓ {output_file} already exists, skipping download")
        return True

    print(f"Downloading {pdb_id} from RCSB PDB...")
    try:
        urllib.request.urlretrieve(url, output_file)
        size_kb = os.path.getsize(output_file) / 1024
        print(f"✓ Downloaded: {output_file} ({size_kb:.1f} KB)")
        return True
    except Exception as e:
        print(f"✗ Failed to download: {e}")
        return False

# ============================================================================
# RUN PARSER
# ============================================================================

def run_parser(pdb_file: str, output_file: str, parser_script: str) -> bool:
    """Execute pdb_to_torsion.py to generate TorsionArray binary."""
    if not Path(parser_script).exists():
        print(f"✗ Parser script not found: {parser_script}")
        return False

    print(f"\nRunning parser: python {parser_script}")
    try:
        result = subprocess.run(
            ["python3", parser_script, "--input", pdb_file, "--output", output_file],
            capture_output=True,
            text=True,
            timeout=30
        )

        print(result.stdout)
        if result.stderr:
            print("STDERR:", result.stderr)

        if result.returncode != 0:
            print(f"✗ Parser failed with exit code {result.returncode}")
            return False

        # Verify output
        if not Path(output_file).exists():
            print(f"✗ Output file not created: {output_file}")
            return False

        size_bytes = os.path.getsize(output_file)
        if size_bytes != 740:
            print(f"✗ Invalid output size: {size_bytes} bytes (expected 740)")
            return False

        print(f"✓ Generated: {output_file} ({size_bytes} bytes)")
        return True

    except subprocess.TimeoutExpired:
        print(f"✗ Parser timed out after 30 seconds")
        return False
    except Exception as e:
        print(f"✗ Parser execution failed: {e}")
        return False

# ============================================================================
# VERIFY INTEGRATION READINESS
# ============================================================================

def verify_integration_readiness(binary_file: str) -> bool:
    """Verify that the test data is ready for integration testing."""
    if not Path(binary_file).exists():
        print(f"✗ Binary not found: {binary_file}")
        return False

    size = os.path.getsize(binary_file)
    if size != 740:
        print(f"✗ Invalid binary size: {size} bytes (expected 740)")
        return False

    # Read and validate basic structure
    try:
        with open(binary_file, 'rb') as f:
            data = f.read()

        # Extract sequence_length (byte 720)
        sequence_length = data[720]
        if sequence_length > 90:
            print(f"✗ Sequence length {sequence_length} exceeds 90-residue limit")
            return False

        # Extract PDB ID (bytes 722-725)
        pdb_id = data[722:726].decode('ascii', errors='ignore').strip('\x00')
        print(f"  PDB ID: {pdb_id}")

        # Extract CRC32 (bytes 734-737, little-endian)
        crc32_bytes = data[734:738]
        crc32 = int.from_bytes(crc32_bytes, byteorder='little')
        print(f"  Sequence length: {sequence_length}")
        print(f"  CRC32: 0x{crc32:08x}")

        return True

    except Exception as e:
        print(f"✗ Failed to validate binary: {e}")
        return False

# ============================================================================
# MAIN EXECUTION
# ============================================================================

def main():
    print("=== Day 8 Tier 3: Integration Test Data Preparation ===\n")

    # Step 1: Download PDB
    print("STEP 1: Download PDB file")
    if not download_pdb(PDB_ID, PDB_URL, PDB_FILE):
        print("✗ Failed to download PDB file")
        return 1

    # Step 2: Verify PDB file
    if not Path(PDB_FILE).exists():
        print("✗ PDB file not available")
        return 1

    print(f"✓ PDB file ready: {PDB_FILE}")

    # Step 3: Run parser
    print("\nSTEP 2: Run PDB-to-Torsion parser")
    if not run_parser(PDB_FILE, TEST_BINARY, PARSER_SCRIPT):
        print("✗ Failed to run parser")
        return 1

    # Step 4: Verify integration readiness
    print("\nSTEP 3: Verify integration readiness")
    if not verify_integration_readiness(TEST_BINARY):
        print("✗ Integration data validation failed")
        return 1

    # Success
    print("\n=== INTEGRATION TEST DATA READY ===")
    print(f"✓ PDB data:  {PDB_FILE}")
    print(f"✓ Torsion binary: {TEST_BINARY}")
    print("\nNext: Run the integration test:")
    print("  cargo test --test test_day8_integration -- --nocapture --ignored")
    print("\nOptional: Visualize Ramachandran plot:")
    print(f"  python {PARSER_SCRIPT} --input {PDB_FILE} --visualize")

    return 0

if __name__ == "__main__":
    sys.exit(main())
