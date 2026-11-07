# Sys4 (System Info Monitor / 시스템 정보 모니터)

`Sys4` is a lightweight Windows system-resource monitor built with **Tauri 2**, **React 19** and **Rust**.

`Sys4`는 **Tauri 2**, **React 19**, **Rust**로 만든 가벼운 Windows 시스템 리소스 모니터입니다.

## Key Features / 주요 기능

- **CPU**: usage, model name, core count, uptime. (사용률·모델명·코어 수·가동 시간)
- **RAM**: used / total / virtual memory, one-click working-set cleanup with freed-memory report. (사용량·가상 메모리, 원클릭 정리 + 확보 용량 표시)
- **GPU**: NVIDIA via NVML (usage, VRAM, temperature, power); **AMD / Intel via DXGI + Windows GPU performance counters** (live usage & VRAM, same source as Task Manager). (NVIDIA는 NVML, AMD/Intel은 작업 관리자와 같은 성능 카운터로 실시간 사용률·VRAM)
- **Network**: live download / upload speed and session totals. (실시간 다운/업 속도, 누적량)
- **Disk**: usage per volume. (드라이브별 사용량)
- **Mini widget**: tiny always-on-top CPU / RAM / VRAM widget. (항상 위 미니 위젯)
- **System tray**: reset position, open main window, toggle widget, quit. (트레이 메뉴)
- **Remembers** window / widget position, collapsed sections, theme and pin state; windows are always kept on-screen after monitor or resolution changes. (위치·접힘·테마 기억, 모니터 변경 시 화면 안으로 자동 복귀)
- **Update check** against GitHub Releases. (GitHub 릴리스 업데이트 알림)

## How to Use / 사용 방법

1. **Sections**: click (or focus + Enter/Space) a card header to expand/collapse it. Collapsed cards still show a live summary. (카드 헤더 클릭/Enter로 접기·펼치기)
2. **Theme**: click the **Sys4** title to switch dark/light. (Sys4 제목 클릭으로 테마 전환)
3. **Widget**: 🖥️ in the title bar, or the tray menu. Click the widget to return to the main window. (🖥️ 버튼 또는 트레이 메뉴, 위젯 클릭 시 메인 창 복귀)
4. **Pin**: 📌 keeps the main window on top. (📌 항상 위)

## Development / 개발

Requirements: Node.js 20+, Rust (stable), Windows 10 1709+ with WebView2.

```bash
npm install
npm run tauri dev        # or run_dev.bat
npm run check            # lint + JS tests + frontend build + Rust tests
build_app.bat            # release build → sys4_vX.Y.Z.exe + NSIS installer in the project root
```

Project layout:

| Path | Purpose |
|---|---|
| `src/` | React UI (`components/`, `hooks/`, `utils/`) |
| `src-tauri/src/system_info.rs` | CPU / RAM / network / disk sampling, RAM cleanup |
| `src-tauri/src/gpu.rs` | NVML + DXGI/PDH GPU telemetry |
| `src-tauri/src/window.rs` | Window placement (multi-monitor safe) and main/widget switching |
| `src-tauri/src/config.rs` | Persistent settings (`%APPDATA%/com.ccoon.sysinfo/config.json`) |
| `CHANGELOG.md` | Release notes |
| `docs/reviews/` | Code review reports |

Release binaries are published on [GitHub Releases](https://github.com/marushin/Sys4/releases), not committed to the repository.

---
© 2026 ccoon. All rights reserved.
