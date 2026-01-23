from __future__ import annotations

import logging
import subprocess  # noqa: S404
from pathlib import Path

# Configure logging
logging.basicConfig(level=logging.INFO, format="%(message)s")
logger = logging.getLogger(__name__)

# Define paths
PROTO_DIR = Path("proto")
COMPILED_PY_DIR = Path("backend/packages/proto/proto/compiled")

# Ensure compiled directories exist
COMPILED_PY_DIR.mkdir(parents=True, exist_ok=True)

# Compile proto files for Python
logger.info("Compiling proto files for Python...")
for proto_file in PROTO_DIR.glob("*.proto"):
    subprocess.run(  # noqa: S603
        [  # noqa: S607
            "python",
            "-m",
            "grpc_tools.protoc",
            f"--proto_path={PROTO_DIR}",
            f"--python_out={COMPILED_PY_DIR}",
            f"--grpc_python_out={COMPILED_PY_DIR}",
            f"--pyi_out={COMPILED_PY_DIR}",
            str(proto_file),
        ],
        check=False,
    )

# Use protol to fix imports
logger.info("Fixing imports for Python...")
subprocess.run(  # noqa: S603
    [  # noqa: S607
        "protol",
        "--create-package",
        "--in-place",
        f"--python-out={COMPILED_PY_DIR}",
        "protoc",
        f"--proto-path={PROTO_DIR}",
        *[str(f) for f in PROTO_DIR.glob("*.proto")],
    ],
    check=False,
)

logger.info("Python proto files compiled to %s with fixed imports", COMPILED_PY_DIR)
