#!/usr/bin/python3

import subprocess
import json


def find_workspace_root():
    """
    Run `cargo metadata` and parse the output to get the workspace root.
    """

    try:
        # Run the "cargo metadata" command and capture its output
        result = subprocess.run(['cargo', 'metadata', '--format-version=1'], capture_output=True, text=True, check=True)

        # Parse the JSON output
        cargo_metadata = json.loads(result.stdout)

        # Extract and print the workspace root
        workspace_root = cargo_metadata['workspace_root']
        print(workspace_root)

    except subprocess.CalledProcessError as e:
        print(f"Error running 'cargo metadata': {e}")

    except json.JSONDecodeError as e:
        print(f"Error decoding JSON output: {e}")


if __name__ == '__main__':
    find_workspace_root()