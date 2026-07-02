# Project Overview
Lightweight mouse gesture application for Windows (Rust + Tauri)

## Features
- Collects mouse trajectories using a global low-level mouse hook
- Gesture recognition using pattern matching
- Sends key inputs or launches applications based on recognition results
- Pattern and action configuration via JSON
- Management of target / ignored windows

## Architecture
- Frontend: Tauri + React + TypeScript
- Backend: Rust + Win32 API

## Structure
- Frontend
  - UI rendering and configuration operations
- Backend
  - Mouse hooks, gesture recognition, command execution, file I/O
- gestures.json
  - Gesture pattern definitions
- config.json
  - Mapping between gestures and actions, global ignored EXE list, trajectory visualization settings

## Coding Rules
1. Every module must include a Japanese header comment.
   - Must include an overview, input/output specifications, and concrete examples.
2. Comments within source code must be kept to a minimum.
   - Comments are allowed only for core function headers.
   - TODO comments are prohibited.
3. Enforce snake_case naming conventions strictly.

The above are mandatory requirements. Violations are not permitted.

## Module Guidelines
- Each module must follow the Single Responsibility Principle.

## Debugging
- Logging is optional and must not output to standard output in release builds.
- Debug helpers must be separated and easily removable.

## Workflow
1. Keep the TODO list visible and manage it continuously.
2. Detect context.
3. Make edits with minimal diffs.
4. Verify through builds.
5. Share progress and propose the next actions.
