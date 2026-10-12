"""Local build inputs; overrides also support installations on another drive."""
from pathlib import Path
import os

ROOT = Path(__file__).resolve().parents[1]
DRIVE = Path(ROOT.anchor)
GAME = Path(os.environ.get('NRHEX_GAME', DRIVE / 'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game'))
SMITHBOX = Path(os.environ.get('NRHEX_SMITHBOX', DRIVE / 'Program Files/Smithbox'))
