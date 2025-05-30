from __future__ import annotations

import logging
import subprocess  # noqa: S404
from pathlib import Path

# Configure logging
logging.basicConfig(level=logging.INFO, format="%(message)s")
logger = logging.getLogger(__name__)

# Define paths
PROTO_DIR = Path("proto")
COMPILED_TS_DIR = Path("packages/proto/src/lib/gen")

# Ensure compiled directory exists
COMPILED_TS_DIR.mkdir(parents=True, exist_ok=True)

# Compile proto files for TypeScript
logger.info("Compiling proto files for TypeScript...")
proto_files = list(PROTO_DIR.glob("*.proto"))
for proto_file in proto_files:
    subprocess.run(  # noqa: S603
        [  # noqa: S607
            "protoc",
            f"--proto_path={PROTO_DIR}",
            "--plugin=node_modules/.bin/protoc-gen-ts_proto",
            f"--js_out=import_style=commonjs:{COMPILED_TS_DIR}",
            f"--grpc-web_out=import_style=typescript,mode=grpcwebtext:{COMPILED_TS_DIR}",
            str(proto_file),
        ],
        check=False,
    )

logger.info("TypeScript proto files compiled to %s", COMPILED_TS_DIR)
