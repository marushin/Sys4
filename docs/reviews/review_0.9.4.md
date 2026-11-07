# Code Review & Refactoring Report — Sys4 v0.9.4

- **리뷰일**: 2026-11-07
- **대상**: v0.9.3 → v0.9.4 (Tauri 2, React 19, Rust, Win32/DXGI/PDH, Vite 7)
- **범위**: 전체 코드 리뷰, 이전 3Eyes 리뷰(`3eyes_review_0816.md`) 미해결 권고 처리, 기능 업그레이드, 최적화, 저장소 정리

---

## 1. 리뷰에서 발견된 결함 (수정 완료)

| # | 결함 | 위치 (v0.9.3) | 재현 조건 | 조치 |
|---|---|---|---|---|
| B1 | **설정 덮어쓰기 레이스**: 각 창이 전체 설정 사본을 저장 → 다른 창/백엔드가 쓴 키가 사라짐 | `src/utils/config.js` `setConfigValue` → `save_app_config(전체 맵)` | 위젯을 옮겨 위치 저장 → 메인 창에서 섹션 접기 → 재실행 시 위젯 위치가 이전 값으로 복원됨 (Rust `ensure_window_visible`이 쓴 좌표도 동일하게 유실) | 백엔드 `ConfigStore`(Mutex) + 부분 병합 커맨드 `update_app_config`, 원자적 저장(temp + rename) |
| B2 | **async 런타임에서 블로킹 호출** | `lib.rs` 모든 `async fn` 커맨드 | RAM 정리(전체 프로세스 순회) 중 폴링이 같은 `Mutex<System>`에서 대기 | 무거운 작업은 `spawn_blocking`, 서브시스템별 개별 락 |
| B3 | **GPU 없는 PC에서 GPU 카드가 영구 표시** | `ResourcePanel.jsx` `sys4_has_gpu` | GPU 없는 환경: 빈 GPU 헤더가 계속 남음 (false로 바뀌는 경로 없음) | `useGpuCache`: 3회 연속 미검출 시 숨김 |
| B4 | **AMD/Intel VRAM 4GB 상한** | `system_info.rs` WMI `AdapterRAM` | 8GB+ Radeon에서 전체 VRAM이 4.0 GB 이하로 표시 (uint32) | DXGI `DedicatedVideoMemory`(64bit)로 교체 |
| B5 | **트레이 "미니 위젯 전환"이 최초에는 동작 안 함** | `lib.rs` `toggle_widget_action` | 앱 시작 후 UI에서 위젯을 한 번도 열지 않은 상태 → 트레이 메뉴로 위젯 전환 불가 (위젯 창이 JS에서만 생성됨) | 위젯 창 생성을 Rust(`window::ensure_widget`)로 이동 |
| B6 | **위젯 기본 위치가 작업표시줄과 겹침** | `move_window_to_bottom_right_inner` | 모니터 전체 높이 기준 -20px → 48px 작업표시줄 위로 겹침 | 모니터 **작업 영역(work area)** 기준 배치·클램핑 |
| B7 | 폴링 중첩 가능 | `setInterval` + async invoke | 백엔드 응답이 2초 이상 지연 시 요청 누적 | 자기 재스케줄 `setTimeout` 체인 |
| B8 | 시작 시 창 점프 | 창이 기본 위치에 먼저 표시 → JS 로드 후 저장 위치로 이동 | 매 실행 | 메인 창 `visible:false` → Rust setup에서 배치 후 표시 |

## 2. 이전 리뷰 권고 사항 처리 현황 (3eyes_review_0816)

| # | 권고 | 상태 |
|---|---|---|
| 5 | 테마 토글 발견성·키보드 접근성 | ✅ `Sys4` 제목을 `<button>`으로 (aria-label, 포커스 링) |
| 6 | IPC 실패 시 피드백 | ✅ "데이터 갱신 실패 · 재시도 중…" 배너 |
| 7 | 메인-위젯 디자인 불일치 | ✅ 위젯 라이트 테마, 경고/위험 색상 공유, 테마 동기화 |
| 8 | 접기/펼치기 키보드 접근성 | ✅ `role=button`, `tabIndex`, `aria-expanded`, Enter/Space |
| 9 | 포커스 링 제거(`outline:none`) | ✅ `:focus-visible` 복원 |
| 10 | 색상 팔레트 혼재(복숭아 테마에 초록 hover) | ✅ `--color-primary-strong`로 통일 |
| 11 | 다크 모드 프로그레스 트랙 대비 | ✅ `--color-track` 토큰 |
| 12 | 업데이트 알림 디버그 색상(네온 초록) | ✅ 테마 토큰 기반 재디자인, 한국어화 |
| 14 | 데드 CSS/토큰 | ✅ statusbar/badge/overlay/hero 등 제거 |
| 15 | 경고/위험 텍스트 AA 대비 미달 | ✅ 라이트 `#B45309`/`#DC2626` 텍스트 전용 토큰 |
| 16 | 브랜딩 통일 | ✅ 창 제목/HTML 제목 `Sys4`, `lang="ko"`, 파비콘 앱 아이콘 |
| 17 | async 위 블로킹 콜 | ✅ B2 |
| 18 | 폴링 로직 중복 | ✅ `useSystemResources`, `useGpuCache`, `useWindowBehavior` 훅 |
| 19 | 디스크 목록 매 폴링 재생성 | ✅ 5초 공간 갱신 / 30초 목록 재탐색 캐시 |
| 20 | 빌드 산출물 저장소 커밋 | ✅ `.gitignore`(*.exe, dist, target), Releases로 배포 |
| 21 | 테스트 전무 | ✅ Rust 18개(+하드웨어 진단 1), JS 8개 |
| 22 | 데드 의존성 | ✅ `tauri-plugin-autostart`(Rust/JS), `wmi` 제거 |
| 23 | RAM 정리 토스트 | ✅ 한국어 + 확보 용량 표시 |
| 28 | 업데이트 알림 거부 영속화 | ✅ 버전별 "숨기기" 저장, pre-release 비교 지원 |
| 29 | `prefers-reduced-motion` | ✅ 메인/위젯 모두 반영 |
| 13 | 이모지 → SVG 아이콘 | ⏸ 보류 (시각 디자인 결정 필요) |
| 26/31 | 단축키, About/자동시작 UI | ⏸ 로드맵 유지 |

## 3. 기능 업그레이드

- **AMD/Intel GPU 실시간 모니터링**: DXGI로 어댑터(이름, 실제 VRAM, LUID) 선택 + PDH `GPU Engine(*)\Utilization Percentage`, `GPU Adapter Memory(*)\Dedicated/Shared Usage`. 작업 관리자와 같은 방식(엔진별 프로세스 합 → 최대 엔진)으로 사용률 산출. 내장 GPU는 공유 메모리 기준.
  - 검증: RTX 5090에서 일반 경로 결과(2.9 / 31.4 GB, 13.6%)가 NVML 결과(3.3 / 31.8 GB, 9%)와 근접.
- **CPU 카드**: CPU 모델명, 시스템 가동 시간.
- **미니 위젯**: CPU 행 추가, 콘텐츠 높이에 맞춘 자동 크기, 사용률 경고 색상, 라이트 테마.
- **RAM 정리 결과**: "N개 정리 · X.X GB 확보".
- **트레이 툴팁**에 버전 표시.

## 4. 최적화

| 항목 | 효과 |
|---|---|
| 디스크 캐시 (5s/30s) | 2초마다 전체 디스크 열거 → 필요 시에만 |
| CPU `refresh_cpu_usage` | 사용률만 갱신 (주파수 조회 제거) |
| 창 리사이즈 | `ResizeObserver` + 높이 변화 시에만 `setSize` IPC |
| 설정 저장 | 값 변경 없으면 디스크 쓰기 생략 |
| 릴리스 프로파일 | `lto`, `codegen-units=1`, `opt-level="s"`, `strip` |
| 번들 대상 | `all`(MSI+NSIS) → `nsis` (배포물과 일치, WiX 빌드 시간 제거) |
| 타이틀바 아이콘 | 1024px 875KB PNG → 64px 8.5KB |
| 폰트 | 동일 가변 폰트 `@font-face` 3개 → 1개 (`font-weight: 100 900`) |
| 빌드 캐시 | 동기화 폴더 내 `src-tauri/target`(5.5 GB) 제거, `%TEMP%\sys4_target` 사용 |

## 5. 보안

- CSP 적용 (`null` → `default-src 'self'` 기반, `connect-src`는 IPC + `api.github.com`만 허용). 인라인 테마 스크립트는 `public/theme-init.js`로 분리.
- 권한 축소: 위젯 생성이 Rust로 이동하여 `allow-create`, `allow-create-webview-window`, `allow-get-all-windows`, `allow-set-position` 등 제거.

## 6. 구조 변경 (Rust)

```
src-tauri/src/
├── lib.rs          커맨드, 트레이, setup
├── config.rs       ConfigStore (병합·원자적 저장) + 테스트
├── window.rs       순수 배치 로직(resolve_position) + Tauri 연결 + 테스트
├── gpu.rs          NVML / DXGI + PDH + 집계 함수 테스트
└── system_info.rs  CPU/RAM/네트워크/디스크, RAM 정리 + 테스트
```

## 7. 검증

- `npm run lint` — 0 error
- `npm test` — 8/8 pass
- `npm run build` — 성공
- `cargo clippy --all-targets` — 0 warning
- `cargo test` — 18 pass (+1 ignored 하드웨어 진단: `cargo test -- --ignored --nocapture gpu_live_sample`)
- `npx tauri build` — NSIS 설치 파일 생성, 실행 확인
