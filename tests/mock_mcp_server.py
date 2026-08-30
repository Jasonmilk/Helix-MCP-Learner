#!/usr/bin/env python3
"""
模拟 MCP Server — 用于 Helix-MCP-Learner 端到端验证
提供 4 个工具：read_file, list_files, create_file, delete_file
"""

import json
import sys
import os

def handle_initialize(params):
    return {
        "protocolVersion": "2024-11-05",
        "capabilities": {"tools": {}},
        "serverInfo": {"name": "mock-mcp-server", "version": "0.1.0"}
    }

def handle_tools_list(params):
    return {
        "tools": [
            {
                "name": "read_file",
                "description": "Read a file from the filesystem",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path to read"}
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "list_files",
                "description": "List files in a directory",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "directory": {"type": "string", "description": "Directory to list"}
                    },
                    "required": ["directory"]
                }
            },
            {
                "name": "create_file",
                "description": "Create a new file with content",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path to create"},
                        "content": {"type": "string", "description": "File content"}
                    },
                    "required": ["path", "content"]
                }
            },
            {
                "name": "delete_file",
                "description": "Delete a file from the filesystem",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path to delete"}
                    },
                    "required": ["path"]
                }
            }
        ]
    }

def handle_tools_call(params):
    name = params.get("name", "")
    args = params.get("arguments", {})

    if name == "read_file":
        path = args.get("path", "")
        try:
            with open(path, "r") as f:
                content = f.read()
            return {"content": [{"type": "text", "text": content}], "isError": False}
        except Exception as e:
            return {"content": [{"type": "text", "text": f"Error: {e}"}], "isError": True}

    elif name == "list_files":
        directory = args.get("directory", ".")
        try:
            files = os.listdir(directory)
            return {"content": [{"type": "text", "text": "\n".join(files)}], "isError": False}
        except Exception as e:
            return {"content": [{"type": "text", "text": f"Error: {e}"}], "isError": True}

    elif name == "create_file":
        path = args.get("path", "")
        content = args.get("content", "")
        try:
            os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
            with open(path, "w") as f:
                f.write(content)
            return {"content": [{"type": "text", "text": f"Created: {path}"}], "isError": False}
        except Exception as e:
            return {"content": [{"type": "text", "text": f"Error: {e}"}], "isError": True}

    elif name == "delete_file":
        path = args.get("path", "")
        try:
            os.remove(path)
            return {"content": [{"type": "text", "text": f"Deleted: {path}"}], "isError": False}
        except Exception as e:
            return {"content": [{"type": "text", "text": f"Error: {e}"}], "isError": True}

    else:
        return {"content": [{"type": "text", "text": f"Unknown tool: {name}"}], "isError": True}

def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            request = json.loads(line)
        except json.JSONDecodeError:
            continue

        method = request.get("method", "")
        params = request.get("params", {})
        req_id = request.get("id")

        # 通知（无 id）不需要响应
        if req_id is None:
            continue

        if method == "initialize":
            result = handle_initialize(params)
        elif method == "tools/list":
            result = handle_tools_list(params)
        elif method == "tools/call":
            result = handle_tools_call(params)
        else:
            result = {"error": {"code": -32601, "message": f"Method not found: {method}"}}

        response = {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": result
        }
        print(json.dumps(response), flush=True)

if __name__ == "__main__":
    main()
