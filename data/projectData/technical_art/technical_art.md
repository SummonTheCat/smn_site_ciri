# Project: Technical Art — Creature Rig v1

> “Make it simple, then make it beautiful.”

## Overview

This document outlines the goals, assets, and technical notes for the **Creature Rig v1** prototype.  
All referenced media files live in the project’s `resources/` directory and are linked using **relative paths** so they resolve under `/projects/technical_art/...`.

---

## Goals

- Establish a flexible biped/quadruped rig preset.
- Support export to engine with minimal baking.
- Provide an animator-friendly picker and control set.
- Document constraints, naming, and export rules.

---

## Tools

- DCC: **Blender 4.2** / **Maya 2023**
- Versioning: **Git LFS**
- Engine: **Unity 2022 LTS**
- Scripting: **Python**, **Rust** (pipeline helpers)

## Rig Specs

- **Skeleton:** 76 deform joints, FK/IK spine, switchable arms/legs
- **Deformations:** Dual Quaternion with corrective shapes
- **Controls:** World/Root, COG, FK/IK limbs, foot roll, twist bones
- **Export:** Deform-only skeleton + baked curves, 30 FPS
