#!/usr/bin/env python3
"""
Convert BAR S3O models to glTF (.glb) format with team coloring.
Batch-converts all unit and building models for both blue and red teams.

Dependencies: pip install pygltflib Pillow numpy
"""

import struct
import os
import sys
import urllib.request
from io import BytesIO
from pathlib import Path

try:
    import pygltflib
    from pygltflib import GLTF2, Scene, Node, Mesh, Primitive, Accessor, BufferView, Buffer, Material, Asset
    from pygltflib import PbrMetallicRoughness
except ImportError:
    print("Error: pygltflib not installed. Run: pip install pygltflib")
    sys.exit(1)

try:
    from PIL import Image
except ImportError:
    Image = None
    print("Warning: Pillow not installed. Textures will not be converted.")
    print("Run: pip install Pillow")

# Base URL for BAR repo
BAR_BASE_URL = "https://raw.githubusercontent.com/beyond-all-reason/Beyond-All-Reason/master"
TEX1_URL = f"{BAR_BASE_URL}/unittextures/Arm_color.dds"

# All models to convert: (output_prefix, s3o_filename)
MODELS = [
    ("armcom",   "armcom.s3o"),    # Commander
    ("armpeep",  "armpeep.s3o"),   # Scout
    ("armflash", "armflash.s3o"),  # Raider
    ("armstump", "armstump.s3o"),  # Tank
    ("armbull",  "armbull.s3o"),   # Assault
    ("armham",   "armham.s3o"),    # Artillery
    ("armmex",   "armmex.s3o"),    # Metal Extractor
    ("armsolar", "armsolar.s3o"),  # Solar Collector
    ("armlab",   "armlab.s3o"),    # Factory
    ("armllt",   "armllt.s3o"),    # LLT
    ("armdrag",  "armdrag.s3o"),   # Dragon's Teeth
    ("armrad",   "armrad.s3o"),    # Radar Tower
]

TEAM_COLORS = {
    "blue": (51, 102, 255),
    "red": (255, 51, 51),
}

SCRIPT_DIR = Path(__file__).parent
PROJECT_DIR = SCRIPT_DIR.parent
OUTPUT_DIR = PROJECT_DIR / "assets" / "models"
CACHE_DIR = SCRIPT_DIR / ".cache"

# Pieces to skip globally: emitter/flare/anchor points with no useful geometry
SKIP_PIECES_GLOBAL = {
    'cagelight_emit', 'armhexl_emit', 'armhexl2_emit',  # Emitter points
    'laserflare', 'lflare', 'nano',                       # Weapon flare points
    'hatpoint', 'rfootstep', 'lfootstep',                  # Anchor points
    'teleport',                                             # Teleport pad (no geometry)
    'flare', 'emit', 'light', 'groundflash',               # Generic emitter names
}

# Commander-specific decorative overlays that Z-fight
SKIP_PIECES_ARMCOM = {
    'medalgold', 'medalbronze', 'medalsilver',  # Stacked medals
    'crown',                                       # Overlaps with head
}


def download_file(url: str, cache_name: str) -> bytes:
    """Download a file, caching it locally."""
    cache_path = CACHE_DIR / cache_name
    if cache_path.exists():
        print(f"  Using cached: {cache_name}")
        return cache_path.read_bytes()

    print(f"  Downloading: {url}")
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
        with urllib.request.urlopen(req) as resp:
            data = resp.read()
        CACHE_DIR.mkdir(parents=True, exist_ok=True)
        cache_path.write_bytes(data)
        return data
    except Exception as e:
        print(f"  Failed to download {url}: {e}")
        return None


# --- S3O Parser ---

class S3OHeader:
    SIZE = 52

    def __init__(self, data: bytes):
        fields = struct.unpack_from("<12s I f f 3f I I I I", data, 0)
        self.magic = fields[0]
        self.version = fields[1]
        self.radius = fields[2]
        self.height = fields[3]
        self.mid_x = fields[4]
        self.mid_y = fields[5]
        self.mid_z = fields[6]
        self.root_piece_offset = fields[7]
        self.collision_data_offset = fields[8]
        self.tex1_name_offset = fields[9]
        self.tex2_name_offset = fields[10]

        if self.magic[:12] != b"Spring unit\x00":
            raise ValueError(f"Not an S3O file: magic = {self.magic!r}")
        print(f"  S3O version: {self.version}")
        print(f"  Radius: {self.radius:.1f}, Height: {self.height:.1f}")
        print(f"  Midpoint: ({self.mid_x:.1f}, {self.mid_y:.1f}, {self.mid_z:.1f})")


class S3OPiece:
    SIZE = 52

    def __init__(self, data: bytes, offset: int):
        fields = struct.unpack_from("<10I 3f", data, offset)
        self.name_offset = fields[0]
        self.num_children = fields[1]
        self.children_offset = fields[2]
        self.num_verts = fields[3]
        self.verts_offset = fields[4]
        self.vertex_type = fields[5]
        self.primitive_type = fields[6]
        self.num_indices = fields[7]
        self.indices_offset = fields[8]
        self.collision_data_offset = fields[9]
        self.x_offset = fields[10]
        self.y_offset = fields[11]
        self.z_offset = fields[12]

        # Read name (null-terminated string)
        if self.name_offset > 0 and self.name_offset < len(data):
            end = data.index(b"\x00", self.name_offset)
            self.name = data[self.name_offset:end].decode("ascii", errors="replace")
        else:
            self.name = "unnamed"

        # Read vertices (32 bytes each: pos xyz, normal xyz, uv)
        self.vertices = []
        for i in range(self.num_verts):
            v_off = self.verts_offset + i * 32
            vx, vy, vz, nx, ny, nz, tu, tv = struct.unpack_from("<8f", data, v_off)
            # Negate X for S3O coordinate convention
            self.vertices.append((-vx, vy, vz, -nx, ny, nz, tu, tv))

        # Read indices (uint32 each)
        self.indices = []
        for i in range(self.num_indices):
            idx = struct.unpack_from("<I", data, self.indices_offset + i * 4)[0]
            self.indices.append(idx)

        # Read children
        self.children = []
        for i in range(self.num_children):
            child_offset_ptr = self.children_offset + i * 4
            child_offset = struct.unpack_from("<I", data, child_offset_ptr)[0]
            child = S3OPiece(data, child_offset)
            self.children.append(child)


def parse_s3o(data: bytes):
    """Parse an S3O file and return header + root piece."""
    header = S3OHeader(data)

    # Read texture names
    tex1_name = ""
    tex2_name = ""
    if header.tex1_name_offset > 0 and header.tex1_name_offset < len(data):
        end = data.index(b"\x00", header.tex1_name_offset)
        tex1_name = data[header.tex1_name_offset:end].decode("ascii", errors="replace")
    if header.tex2_name_offset > 0 and header.tex2_name_offset < len(data):
        end = data.index(b"\x00", header.tex2_name_offset)
        tex2_name = data[header.tex2_name_offset:end].decode("ascii", errors="replace")

    print(f"  Tex1: {tex1_name}, Tex2: {tex2_name}")

    root = S3OPiece(data, header.root_piece_offset)
    return header, root, tex1_name, tex2_name


def count_pieces(piece):
    """Count total pieces in hierarchy."""
    count = 1
    for child in piece.children:
        count += count_pieces(child)
    return count


def print_piece_tree(piece, indent=0):
    """Print piece hierarchy."""
    prefix = "  " * indent
    print(f"  {prefix}{piece.name}: {piece.num_verts} verts, {piece.num_indices} indices, "
          f"offset=({piece.x_offset:.1f}, {piece.y_offset:.1f}, {piece.z_offset:.1f})")
    for child in piece.children:
        print_piece_tree(child, indent + 1)


# --- glTF Builder ---

def apply_team_color(tex_data: bytes, team_color=(51, 102, 255), output_size=512):
    """Apply team color to the texture using the alpha channel as a mask.

    In Spring engine: alpha=255 means full team color, alpha=0 means original texture.
    Returns an RGB PNG image as bytes, resized to output_size x output_size.
    """
    if Image is None:
        return None

    import numpy as np

    img = Image.open(BytesIO(tex_data)).convert("RGBA")
    arr = np.array(img, dtype=np.float32)

    # Alpha channel = team color blend factor
    t = arr[:, :, 3:4] / 255.0
    tc = np.array(team_color, dtype=np.float32).reshape(1, 1, 3)
    rgb = arr[:, :, :3] * (1 - t) + tc * t
    result = np.clip(rgb, 0, 255).astype(np.uint8)

    out_img = Image.fromarray(result, "RGB")
    out_img = out_img.resize((output_size, output_size), Image.LANCZOS)

    buf = BytesIO()
    out_img.save(buf, format="PNG", optimize=True)
    return buf.getvalue()


def build_gltf(header, root_piece, tex_png_data=None, model_name="armcom"):
    """Build a glTF2 object from the S3O piece hierarchy."""
    gltf = GLTF2()
    gltf.asset = Asset(version="2.0", generator="convert_s3o.py")
    gltf.scene = 0
    gltf.scenes = [Scene(nodes=[])]
    gltf.nodes = []
    gltf.meshes = []
    gltf.accessors = []
    gltf.bufferViews = []
    gltf.buffers = []
    gltf.images = []
    gltf.textures = []

    # Single textured material (all pieces share one UV atlas)
    if tex_png_data is not None:
        from pygltflib import Texture as GltfTexture, Image as GltfImage, TextureInfo
        gltf.images.append(GltfImage(
            mimeType="image/png",
            name="Arm_color",
        ))
        gltf.textures.append(GltfTexture(source=0, name="Arm_color"))
        gltf.materials = [Material(
            pbrMetallicRoughness=PbrMetallicRoughness(
                baseColorTexture=TextureInfo(index=0),
                metallicFactor=0.3,
                roughnessFactor=0.7,
            ),
            name=f"{model_name}_textured",
        )]
    else:
        # Fallback: solid blue if texture not available
        gltf.materials = [Material(
            pbrMetallicRoughness=PbrMetallicRoughness(
                baseColorFactor=[0.3, 0.5, 0.9, 1.0],
                metallicFactor=0.3,
                roughnessFactor=0.7,
            ),
            name=f"{model_name}_material",
        )]

    # Collect all binary data
    all_bin = bytearray()

    # Build skip set for this model
    skip_pieces = set(SKIP_PIECES_GLOBAL)
    if model_name == "armcom":
        skip_pieces |= SKIP_PIECES_ARMCOM

    def should_skip(piece_name):
        """Check if a piece should be skipped (exact match or suffix match for emitters)."""
        if piece_name in skip_pieces:
            return True
        # Skip pieces ending with common emitter suffixes
        lower = piece_name.lower()
        for suffix in ('_emit', '_flare', 'flare', 'flash'):
            if lower.endswith(suffix):
                return True
        return False

    def add_piece(piece, parent_node_idx=None):
        """Recursively add a piece as a glTF node+mesh."""
        if should_skip(piece.name):
            return

        node_idx = len(gltf.nodes)

        node = Node(
            name=piece.name,
            translation=[piece.x_offset, piece.y_offset, piece.z_offset],
            children=[],
        )

        if piece.num_verts > 0 and piece.num_indices >= 3:
            # Build vertex buffer: interleaved pos(3f) + normal(3f) + uv(2f) = 32 bytes
            vert_data = bytearray()
            min_pos = [float("inf")] * 3
            max_pos = [float("-inf")] * 3

            # Shrink vertices toward local center to reduce Z-fighting at joints.
            SHRINK = 0.95

            # x_clamp only for armcom thigh pieces
            x_clamp = None
            if model_name == "armcom":
                if piece.name == 'lthigh':
                    x_clamp = ('min', -1.0)
                elif piece.name == 'rthigh':
                    x_clamp = ('max', 1.0)

            cx = sum(v[0] for v in piece.vertices) / max(len(piece.vertices), 1)
            cy = sum(v[1] for v in piece.vertices) / max(len(piece.vertices), 1)
            cz = sum(v[2] for v in piece.vertices) / max(len(piece.vertices), 1)

            for vx, vy, vz, nx, ny, nz, tu, tv in piece.vertices:
                sx = cx + (vx - cx) * SHRINK
                sy = cy + (vy - cy) * SHRINK
                sz = cz + (vz - cz) * SHRINK
                if x_clamp is not None:
                    if x_clamp[0] == 'min':
                        sx = max(sx, x_clamp[1])
                    else:
                        sx = min(sx, x_clamp[1])
                vert_data.extend(struct.pack("<3f", sx, sy, sz))
                vert_data.extend(struct.pack("<3f", nx, ny, nz))
                vert_data.extend(struct.pack("<2f", tu, tv))
                for i, val in enumerate([sx, sy, sz]):
                    min_pos[i] = min(min_pos[i], val)
                    max_pos[i] = max(max_pos[i], val)

            # Build index buffer — swap winding order to compensate for X negation.
            # Negating X flips the handedness, so we swap v1/v2 in each triangle.
            idx_data = bytearray()
            for i in range(0, len(piece.indices), 3):
                idx_data.extend(struct.pack("<I", piece.indices[i]))
                idx_data.extend(struct.pack("<I", piece.indices[i + 2]))  # swapped
                idx_data.extend(struct.pack("<I", piece.indices[i + 1]))  # swapped

            # Pad to 4-byte alignment
            while len(all_bin) % 4 != 0:
                all_bin.append(0)

            # Add vertex buffer view
            vert_bv_offset = len(all_bin)
            all_bin.extend(vert_data)
            vert_bv_idx = len(gltf.bufferViews)
            gltf.bufferViews.append(BufferView(
                buffer=0,
                byteOffset=vert_bv_offset,
                byteLength=len(vert_data),
                byteStride=32,  # 3f + 3f + 2f = 32 bytes
                target=pygltflib.ARRAY_BUFFER,
            ))

            # Pad for index buffer
            while len(all_bin) % 4 != 0:
                all_bin.append(0)

            # Add index buffer view
            idx_bv_offset = len(all_bin)
            all_bin.extend(idx_data)
            idx_bv_idx = len(gltf.bufferViews)
            gltf.bufferViews.append(BufferView(
                buffer=0,
                byteOffset=idx_bv_offset,
                byteLength=len(idx_data),
                target=pygltflib.ELEMENT_ARRAY_BUFFER,
            ))

            # Accessors
            pos_acc_idx = len(gltf.accessors)
            gltf.accessors.append(Accessor(
                bufferView=vert_bv_idx,
                byteOffset=0,
                componentType=pygltflib.FLOAT,
                count=piece.num_verts,
                type="VEC3",
                max=max_pos,
                min=min_pos,
            ))

            norm_acc_idx = len(gltf.accessors)
            gltf.accessors.append(Accessor(
                bufferView=vert_bv_idx,
                byteOffset=12,  # after position (3 floats)
                componentType=pygltflib.FLOAT,
                count=piece.num_verts,
                type="VEC3",
            ))

            uv_acc_idx = len(gltf.accessors)
            gltf.accessors.append(Accessor(
                bufferView=vert_bv_idx,
                byteOffset=24,  # after position + normal (6 floats)
                componentType=pygltflib.FLOAT,
                count=piece.num_verts,
                type="VEC2",
            ))

            idx_acc_idx = len(gltf.accessors)
            gltf.accessors.append(Accessor(
                bufferView=idx_bv_idx,
                byteOffset=0,
                componentType=pygltflib.UNSIGNED_INT,
                count=piece.num_indices,
                type="SCALAR",
            ))

            # Mesh
            mesh_idx = len(gltf.meshes)
            gltf.meshes.append(Mesh(
                name=piece.name,
                primitives=[Primitive(
                    attributes=pygltflib.Attributes(
                        POSITION=pos_acc_idx,
                        NORMAL=norm_acc_idx,
                        TEXCOORD_0=uv_acc_idx,
                    ),
                    indices=idx_acc_idx,
                    material=0,
                )],
            ))

            node.mesh = mesh_idx

        gltf.nodes.append(node)

        if parent_node_idx is not None:
            gltf.nodes[parent_node_idx].children.append(node_idx)
        else:
            gltf.scenes[0].nodes.append(node_idx)

        # Add children
        for child in piece.children:
            add_piece(child, node_idx)

    add_piece(root_piece)

    # Embed texture image data if available
    if tex_png_data is not None:
        # Pad to 4-byte alignment
        while len(all_bin) % 4 != 0:
            all_bin.append(0)
        img_offset = len(all_bin)
        all_bin.extend(tex_png_data)
        img_bv_idx = len(gltf.bufferViews)
        gltf.bufferViews.append(BufferView(
            buffer=0,
            byteOffset=img_offset,
            byteLength=len(tex_png_data),
        ))
        gltf.images[0].bufferView = img_bv_idx

    # Set buffer
    gltf.buffers = [Buffer(byteLength=len(all_bin))]
    gltf.set_binary_blob(bytes(all_bin))

    return gltf


def convert_model(model_name, s3o_filename, tex_data):
    """Convert a single S3O model to blue and red GLB variants."""
    model_url = f"{BAR_BASE_URL}/objects3d/Units/{s3o_filename}"
    cache_name = s3o_filename

    print(f"\n{'='*60}")
    print(f"Converting: {model_name} ({s3o_filename})")
    print(f"{'='*60}")

    # Download model
    s3o_data = download_file(model_url, cache_name)
    if s3o_data is None:
        print(f"  FAILED to download {s3o_filename}, skipping.")
        return False

    # Parse S3O
    try:
        header, root_piece, tex1, tex2 = parse_s3o(s3o_data)
        total_pieces = count_pieces(root_piece)
        print(f"  Total pieces: {total_pieces}")
        print("\n  Piece hierarchy:")
        print_piece_tree(root_piece)
    except Exception as e:
        print(f"  FAILED to parse {s3o_filename}: {e}")
        return False

    # Generate both team color variants
    for color_name, team_color in TEAM_COLORS.items():
        tex_png = None
        if tex_data is not None and Image is not None:
            print(f"\n  Applying {color_name} team color...")
            tex_png = apply_team_color(tex_data, team_color=team_color)
            if tex_png:
                print(f"  Texture: {len(tex_png)} bytes PNG")
        else:
            print("  Warning: No texture, using solid color fallback")

        print(f"  Building glTF ({color_name})...")
        gltf = build_gltf(header, root_piece, tex_png, model_name=model_name)

        output_path = OUTPUT_DIR / f"{model_name}_{color_name}.glb"
        gltf.save_binary(str(output_path))

        file_size = output_path.stat().st_size
        print(f"  Saved: {output_path.name} ({file_size/1024:.1f} KB)")

    return True


def main():
    print("=== BAR S3O to GLB Batch Converter ===\n")

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    # Download shared texture (all ARM models use the same atlas)
    print("Downloading shared texture (Arm_color.dds)...")
    tex_data = download_file(TEX1_URL, "Arm_color.dds")

    success = 0
    failed = 0

    for model_name, s3o_filename in MODELS:
        if convert_model(model_name, s3o_filename, tex_data):
            success += 1
        else:
            failed += 1

    print(f"\n{'='*60}")
    print(f"Done! {success} models converted, {failed} failed.")
    print(f"Output: {OUTPUT_DIR}")
    print(f"Total GLB files: {success * 2}")


if __name__ == "__main__":
    main()
