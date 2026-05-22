#!/usr/bin/env python3
"""
pdb_to_torsion.py - PDB to Torsion Array Converter

Day 8 Tier 2: User-Layer Molecular Parser
Extracts backbone dihedral angles (φ/ψ) from PDB files and serializes to
740-byte TorsionArray binary format for supervisor injection.

DESIGN PRINCIPLES:
1. Bit-Identical CRC32: Matches Rust supervisor (polynomial 0xEDB88320)
2. Dihedral Formula: Praxeolitic (atan2-based) for deterministic results
3. 90-Residue Limit: Hard truncation per v3.4 spec
4. Model Selection: Default to model 0 (primary structure)
5. Graceful Degradation: Skip residues with missing backbone atoms

CYCLE COST: ~10ms (offline, no frame constraint)

USAGE:
    python pdb_to_torsion.py --input 1MBN.pdb --chain A --output torsion.bin
    python pdb_to_torsion.py --input 1MBN.pdb --visualize  # With Ramachandran plot
"""

import struct
import argparse
import sys
import math
import zlib
from typing import List, Tuple, Optional, Dict
from pathlib import Path

# Optional imports for visualization
try:
    import numpy as np
    HAS_NUMPY = True
except ImportError:
    HAS_NUMPY = False

try:
    from Bio import PDB
    HAS_BIOPYTHON = True
except ImportError:
    HAS_BIOPYTHON = False

try:
    import matplotlib.pyplot as plt
    import seaborn as sns
    HAS_MATPLOTLIB = True
except ImportError:
    HAS_MATPLOTLIB = False

# ============================================================================
# CONSTANTS & CONFIGURATION
# ============================================================================

TORSION_ARRAY_SIZE = 740  # bytes
MAX_RESIDUES = 90
PI = math.pi
RADIANS_PER_DEGREE = PI / 180.0
DEGREES_PER_RADIAN = 180.0 / PI

# CRC32 polynomial (must match Rust supervisor)
CRC32_POLYNOMIAL = 0xEDB88320
CRC32_INITIAL = 0xFFFFFFFF

# ============================================================================
# VECTOR & DIHEDRAL MATH (Deterministic)
# ============================================================================

def vector_subtract(v1: Tuple[float, float, float],
                    v2: Tuple[float, float, float]) -> Tuple[float, float, float]:
    """Subtract two 3D vectors: v1 - v2"""
    return (v1[0] - v2[0], v1[1] - v2[1], v1[2] - v2[2])

def vector_cross(v1: Tuple[float, float, float],
                 v2: Tuple[float, float, float]) -> Tuple[float, float, float]:
    """Cross product: v1 × v2"""
    return (
        v1[1] * v2[2] - v1[2] * v2[1],
        v1[2] * v2[0] - v1[0] * v2[2],
        v1[0] * v2[1] - v1[1] * v2[0],
    )

def vector_dot(v1: Tuple[float, float, float],
               v2: Tuple[float, float, float]) -> float:
    """Dot product: v1 · v2"""
    return v1[0] * v2[0] + v1[1] * v2[1] + v1[2] * v2[2]

def vector_norm_squared(v: Tuple[float, float, float]) -> float:
    """Squared magnitude: ||v||²"""
    return vector_dot(v, v)

def dihedral_angle(pos_a: Tuple[float, float, float],
                   pos_b: Tuple[float, float, float],
                   pos_c: Tuple[float, float, float],
                   pos_d: Tuple[float, float, float]) -> float:
    """
    Compute dihedral angle A-B-C-D using praxeolitic formula.

    Formula:
        φ = atan2(n1·(n2×b3), n1·n2)
        where n1 = b1×b2, n2 = b2×b3

    Args:
        pos_*: (x, y, z) coordinates of atoms A, B, C, D

    Returns:
        Dihedral angle in [-π, π] radians
        Returns 0.0 if atoms are collinear (degenerate case)
    """
    b1 = vector_subtract(pos_b, pos_a)
    b2 = vector_subtract(pos_c, pos_b)
    b3 = vector_subtract(pos_d, pos_c)

    n1 = vector_cross(b1, b2)
    n2 = vector_cross(b2, b3)

    # Check for degeneracy (collinear atoms)
    n1_norm_sq = vector_norm_squared(n1)
    n2_norm_sq = vector_norm_squared(n2)

    if n1_norm_sq < 1e-10 or n2_norm_sq < 1e-10:
        return 0.0  # Degenerate: return 0

    # Compute atan2(n1·(n2×b3), n1·n2)
    n2_cross_b3 = vector_cross(n2, b3)
    numerator = vector_dot(n1, n2_cross_b3)
    denominator = vector_dot(n1, n2)

    angle = math.atan2(numerator, denominator)
    return angle

def normalize_angle(angle: float) -> float:
    """Normalize angle to [-π, π] using branchless fmod."""
    TWO_PI = 2.0 * PI
    # Reduce to [0, 2π]
    reduced = angle - TWO_PI * math.floor(angle / TWO_PI)
    # Map [0, 2π] → [-π, π] (branchless)
    normalized = reduced - TWO_PI * (1 if reduced > PI else 0)
    return normalized

# ============================================================================
# CRC32 CHECKSUM (Bit-Identical with Rust Supervisor)
# ============================================================================

def crc32_checksum(data: bytes) -> int:
    """
    Compute CRC32 checksum using polynomial 0xEDB88320.
    Must match Rust supervisor exactly for verification.

    Args:
        data: Byte sequence to checksum

    Returns:
        CRC32 value as unsigned 32-bit integer
    """
    crc = CRC32_INITIAL

    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ CRC32_POLYNOMIAL
            else:
                crc >>= 1

    return crc ^ 0xFFFFFFFF

# ============================================================================
# TORSION ARRAY SERIALIZATION
# ============================================================================

class TorsionArray:
    """
    740-byte FFI-compatible torsion array structure.

    Layout (bytes):
        0-719:    angles[180] (f32 × 180)
        720:      sequence_length (u8)
        721:      source_flags (u8)
        722-725:  pdb_id[4] (char × 4)
        726-733:  timestamp_us (u64)
        734-737:  crc32 (u32)
        738-739:  _padding (u16)
    """

    def __init__(self):
        self.angles: List[float] = [0.0] * 180
        self.sequence_length: int = 0
        self.source_flags: int = 0  # Bit 0: from_pdb, Bit 1: has_disorder
        self.pdb_id: bytes = b'\x00\x00\x00\x00'
        self.timestamp_us: int = 0
        self.crc32: int = 0

    def serialize(self) -> bytes:
        """Serialize to 740-byte binary format."""
        # Pack angles as f32 (little-endian)
        angles_bytes = b''.join(struct.pack('<f', a) for a in self.angles)

        # Pack struct fields
        result = (
            angles_bytes +  # 720 bytes
            struct.pack('B', self.sequence_length) +  # 1 byte
            struct.pack('B', self.source_flags) +      # 1 byte
            self.pdb_id[:4].ljust(4, b'\x00') +        # 4 bytes
            struct.pack('<Q', self.timestamp_us) +     # 8 bytes (little-endian u64)
            struct.pack('<I', self.crc32) +            # 4 bytes (little-endian u32)
            struct.pack('<H', 0)                       # 2 bytes padding
        )

        assert len(result) == TORSION_ARRAY_SIZE, f"Size mismatch: {len(result)}"
        return result

    def compute_crc32(self) -> int:
        """Compute CRC32 over active angles only."""
        num_angles = (self.sequence_length * 2)
        active_angles_bytes = b''.join(
            struct.pack('<f', self.angles[i]) for i in range(num_angles)
        )
        return crc32_checksum(active_angles_bytes)

    def finalize(self):
        """Compute and set CRC32 before serialization."""
        self.crc32 = self.compute_crc32()

# ============================================================================
# PDB PARSING (BioPython Wrapper)
# ============================================================================

def extract_backbone_atoms(pdb_file: str,
                          chain_id: str = 'A',
                          model_id: int = 0) -> Dict[int, Dict[str, Tuple[float, float, float]]]:
    """
    Extract backbone atoms (N, CA, C, O) from PDB file.

    Args:
        pdb_file: Path to PDB file
        chain_id: Chain identifier (default 'A')
        model_id: Model index (default 0, first model)

    Returns:
        Dict mapping residue_id → {atom_name → (x, y, z)}

    Raises:
        ValueError: If BioPython not available or file not found
    """
    if not HAS_BIOPYTHON:
        raise ValueError("BioPython not available. Install: pip install biopython")

    parser = PDB.PDBParser(QUIET=True)
    try:
        structure = parser.get_structure('protein', pdb_file)
    except Exception as e:
        raise ValueError(f"Failed to parse PDB: {e}")

    backbone_atoms = {}
    model = structure[model_id]
    chain = model[chain_id]

    for residue in chain:
        res_id = residue.id[1]  # Residue sequence number

        # Hard truncate at 90 residues
        if res_id > MAX_RESIDUES:
            break

        atoms = {}
        for atom_name in ['N', 'CA', 'C', 'O']:
            if atom_name in residue:
                atom = residue[atom_name]
                atoms[atom_name] = tuple(atom.coord)

        # Only include residues with all 4 backbone atoms
        if len(atoms) == 4:
            backbone_atoms[res_id] = atoms

    return backbone_atoms

def compute_dihedrals(backbone_atoms: Dict[int, Dict[str, Tuple[float, float, float]]]) \
        -> List[Tuple[int, float, float]]:
    """
    Compute φ/ψ dihedrals for all residues.

    Args:
        backbone_atoms: Dict from extract_backbone_atoms

    Returns:
        List of (residue_id, phi, psi) tuples
    """
    residue_ids = sorted(backbone_atoms.keys())
    dihedrals = []

    for i, res_id in enumerate(residue_ids):
        atoms = backbone_atoms[res_id]

        # PHI angle: C_{i-1} - N_i - CA_i - C_i
        if i > 0:
            prev_res_id = residue_ids[i - 1]
            prev_atoms = backbone_atoms[prev_res_id]
            phi = dihedral_angle(
                prev_atoms['C'],
                atoms['N'],
                atoms['CA'],
                atoms['C'],
            )
        else:
            phi = 0.0  # No phi for first residue

        # PSI angle: N_i - CA_i - C_i - N_{i+1}
        if i < len(residue_ids) - 1:
            next_res_id = residue_ids[i + 1]
            next_atoms = backbone_atoms[next_res_id]
            psi = dihedral_angle(
                atoms['N'],
                atoms['CA'],
                atoms['C'],
                next_atoms['N'],
            )
        else:
            psi = 0.0  # No psi for last residue

        # Normalize angles to [-π, π]
        phi = normalize_angle(phi)
        psi = normalize_angle(psi)

        dihedrals.append((res_id, phi, psi))

    return dihedrals

# ============================================================================
# VISUALIZATION (Optional Ramachandran Plot)
# ============================================================================

def plot_ramachandran(dihedrals: List[Tuple[int, float, float]],
                      output_file: Optional[str] = None):
    """
    Plot Ramachandran map of φ/ψ angles.

    Args:
        dihedrals: List of (residue_id, phi, psi) tuples
        output_file: Optional path to save figure
    """
    if not HAS_MATPLOTLIB or not HAS_NUMPY:
        print("WARNING: Matplotlib or NumPy not available. Skipping visualization.")
        return

    try:
        # Extract angles
        phis = [d[1] * DEGREES_PER_RADIAN for d in dihedrals]
        psis = [d[2] * DEGREES_PER_RADIAN for d in dihedrals]

        # Create figure
        fig, ax = plt.subplots(figsize=(10, 10))

        # Plot Ramachandran regions (simplified)
        # α-helix region (blue)
        ax.add_patch(plt.Rectangle((-100, -60), 40, 30,
                                   color='lightblue', alpha=0.3, label='α-helix'))

        # β-sheet region (green)
        ax.add_patch(plt.Rectangle((-180, 120), 60, 40,
                                   color='lightgreen', alpha=0.3, label='β-sheet'))

        # Plot data points
        ax.scatter(phis, psis, c='red', s=100, alpha=0.6, edgecolors='darkred')

        # Labels and formatting
        ax.set_xlabel('φ (degrees)', fontsize=12)
        ax.set_ylabel('ψ (degrees)', fontsize=12)
        ax.set_title('Ramachandran Plot (Backbone Dihedrals)', fontsize=14)
        ax.set_xlim(-180, 180)
        ax.set_ylim(-180, 180)
        ax.grid(True, alpha=0.3)
        ax.legend()

        if output_file:
            plt.savefig(output_file, dpi=150)
            print(f"Ramachandran plot saved: {output_file}")
        else:
            plt.show()

        plt.close()

    except Exception as e:
        print(f"WARNING: Failed to plot Ramachandran: {e}")

# ============================================================================
# MAIN CONVERSION PIPELINE
# ============================================================================

def pdb_to_torsion(pdb_file: str,
                   chain_id: str = 'A',
                   model_id: int = 0,
                   output_file: Optional[str] = None) -> TorsionArray:
    """
    Main conversion pipeline: PDB → TorsionArray.

    Args:
        pdb_file: Input PDB file
        chain_id: Chain to extract (default 'A')
        model_id: Model index (default 0)
        output_file: Optional output path for binary serialization

    Returns:
        TorsionArray object (ready for supervisor injection)
    """
    print(f"Parsing PDB: {pdb_file}")

    # Step 1: Extract backbone atoms
    backbone_atoms = extract_backbone_atoms(pdb_file, chain_id, model_id)
    print(f"  Extracted {len(backbone_atoms)} residues with complete backbone")

    # Step 2: Compute dihedrals
    dihedrals = compute_dihedrals(backbone_atoms)
    print(f"  Computed {len(dihedrals)} φ/ψ angles")

    # Step 3: Build TorsionArray
    torsion_array = TorsionArray()
    torsion_array.sequence_length = len(dihedrals)
    torsion_array.source_flags = 1  # Bit 0: from_pdb = 1

    # Mark as truncated if > 90 residues
    if len(dihedrals) > MAX_RESIDUES:
        torsion_array.source_flags |= 2  # Bit 1: has_disorder = 1
        print(f"  WARNING: Sequence has {len(dihedrals)} residues, hard truncating to 90")
        torsion_array.sequence_length = MAX_RESIDUES
        dihedrals = dihedrals[:MAX_RESIDUES]

    # Populate angles
    for i, (res_id, phi, psi) in enumerate(dihedrals):
        torsion_array.angles[2 * i] = phi
        torsion_array.angles[2 * i + 1] = psi

    # PDB ID (extract from filename)
    pdb_id_str = Path(pdb_file).stem.upper()[:4]
    torsion_array.pdb_id = pdb_id_str.encode('ascii').ljust(4, b'\x00')

    # Timestamp (current Unix time in microseconds)
    import time
    torsion_array.timestamp_us = int(time.time() * 1e6)

    # Compute CRC32
    torsion_array.finalize()

    print(f"  TorsionArray finalized:")
    print(f"    sequence_length: {torsion_array.sequence_length}")
    print(f"    source_flags: 0x{torsion_array.source_flags:02x}")
    print(f"    pdb_id: {torsion_array.pdb_id}")
    print(f"    crc32: 0x{torsion_array.crc32:08x}")

    # Step 4: Serialize (if output file specified)
    if output_file:
        binary_data = torsion_array.serialize()
        with open(output_file, 'wb') as f:
            f.write(binary_data)
        print(f"  Serialized to {output_file} ({len(binary_data)} bytes)")

    return torsion_array

# ============================================================================
# CLI INTERFACE
# ============================================================================

def main():
    parser = argparse.ArgumentParser(
        description='Extract backbone dihedrals from PDB and serialize to TorsionArray',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python pdb_to_torsion.py --input 1MBN.pdb --output torsion.bin
  python pdb_to_torsion.py --input 1MBN.pdb --visualize
  python pdb_to_torsion.py --input 1MBN.pdb --chain B --model 1
        """
    )

    parser.add_argument('--input', '-i', required=True,
                        help='Input PDB file')
    parser.add_argument('--output', '-o', default=None,
                        help='Output binary file (740 bytes TorsionArray)')
    parser.add_argument('--chain', '-c', default='A',
                        help='Chain ID to extract (default: A)')
    parser.add_argument('--model', '-m', type=int, default=0,
                        help='Model index (default: 0, first model)')
    parser.add_argument('--visualize', '-v', action='store_true',
                        help='Plot Ramachandran map of extracted angles')

    args = parser.parse_args()

    try:
        # Convert PDB to TorsionArray
        torsion_array = pdb_to_torsion(
            args.input,
            chain_id=args.chain,
            model_id=args.model,
            output_file=args.output
        )

        # Optional visualization
        if args.visualize:
            # Extract dihedrals for plotting
            backbone_atoms = extract_backbone_atoms(args.input, args.chain, args.model)
            dihedrals = compute_dihedrals(backbone_atoms)
            plot_ramachandran(dihedrals, output_file='ramachandran.png')

        print("\n✅ SUCCESS: TorsionArray generation complete")
        print(f"Ready for supervisor injection: inject_torsion_array(state, array)")

        return 0

    except Exception as e:
        print(f"\n❌ ERROR: {e}", file=sys.stderr)
        return 1

if __name__ == '__main__':
    sys.exit(main())
