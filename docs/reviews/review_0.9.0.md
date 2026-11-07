# Code Review & Refactoring Report - sysinfo-app (Sys4) v0.9.0

**릴리즈 일시**: 2026-08-20  
**앱 버전**: v0.9.0  
**기술 스택**: Tauri v2, React 19, Rust, Windows Win32 API, Vite  

---

## 📋 v0.9.0 주요 개요

v0.9.0은 3관점 종합 리뷰(`3eyes_review_0816.md`)에서 제기된 P0(안정성/정확성 버그) 및 핵심 UX/UI 개선 사항을 완벽히 수렴하여, 안정성과 시각적 완성도, 사용자 편의성을 크게 끌어올린 메이저 마이너 릴리즈입니다.

---

## ✅ 핵심 변경 및 개선 내역

### 1. 🛡️ 안정성 및 크래시 방지
* **싱글 인스턴스 패닉 방지**: `src-tauri/src/lib.rs`의 `expect`를 안전한 `if let Some` 구조로 변경하고, 중복 실행 시 기존 창을 안전하게 `show() + unminimize() + set_focus()`하도록 수정.
* **위젯 닫기 안전화**: `src/components/Widget.jsx`에서 ✕ 클릭 시 `mainWindow.close()`를 호출하던 문제를 제거하고 위젯 hide 및 메인 창 복귀로 프로세스 안정성 확보.

### 2. ⚡ 데이터 측정 정확도 고도화
* **정밀 경과 시간(`Instant`) 기반 네트워크 속도 산출**: `src-tauri/src/system_info.rs`에 `NetworkTracker`를 도입하여 고정 2.0s 분모 대신 실제 경과 시간(`elapsed`)을 측정하여 나눔으로써 왜곡 없는 KB/s / MB/s 속도 제공.
* **WMI GPU 수치 왜곡 방지**: `Option<f64>` 타입을 적용하여 실시간 측정이 불가능한 환경에서는 `0.0%`가 아닌 `None`을 반환하고 UI에 `—`로 정직하게 표시.

### 3. 💾 영구 스토리지 및 창 위치 기억 (Persistence & Window Position)
* **Rust 백엔드 기반 영구 설정 스토리지(`config.json`)**: WebView2 캐시 유실 문제를 방지하기 위해 `%APPDATA%/com.ccoon.sysinfo/config.json`을 직접 읽고 쓰는 백엔드 IPC 커맨드를 도입.
* **메인 창 및 위젯 화면 좌표 실시간 기억**: 창 이동 이벤트(`onMoved`)를 디바운스하여 `main_window_pos_x/y`, `widget_pos_x/y`로 저장하고, 앱 재실행 및 위젯 전환 시 정확한 좌표로 복원.
* **카드 접힘 상태 영구 보존**: `sys4_expanded_sections`를 통해 사용자가 지정한 접힘/펼침 상태를 보존하며, 접힌 카드는 백그라운드 리소스 조회를 생략하여 성능 향상.
* **테마 및 항상 위 고정 상태 영구 유지**: `theme`, `sys4_always_on_top` 상태를 보존.

### 4. 🎨 UI / UX 디테일 & Polish
* **RAM 캐시 정리 인터페이스**: RAM 카드 헤더 내 마이크로 액션 버튼(`🧹`) 및 한국어 미니 토스트 배지로 슬림화.
* **항상 위 핀 버튼 인지성 강화**: 비설정 시 무채색(`grayscale`) ↔ 설정 시 선명한 원색 전환으로 직관적인 상태 피드백 제공.
* **레이아웃 여백 및 호흡감 복원**: 카드 간격(`10px`), 카드 내부 패딩(`10px 12px`), 게이지 바 및 행 간격을 정돈하여 시각적 피로도 감소.

---

## 🛠️ 상세 수정 파일 목록

| 영역 | 파일 | 수정 내용 |
|:---|:---|:---|
| **설정** | `package.json`, `Cargo.toml`, `tauri.conf.json` | 버전을 **v0.9.0**으로 상향 동기화 |
| **Rust 백엔드** | `src-tauri/src/lib.rs` | 싱글 인스턴스 안전 복원, `NetworkTracker`, `get_app_config`/`save_app_config` 구현 |
| **Rust 백엔드** | `src-tauri/src/system_info.rs` | 네트워크 `Instant` 실측 계산, `GpuInfo Option<f64>` 적용, 정리 메시지 한글화 |
| **공통 유틸** | `src/utils/config.js` | Rust 백엔드 `config.json` 동기화 및 캐싱 유틸리티 |
| **React 프론트** | `src/App.jsx` | 창 위치 기억 및 복원, 카드 접힘 상태 영속화 |
| **React 프론트** | `src/components/TitleBar.jsx` | 테마/핀/위젯 위치 복원 및 상태 동기화 |
| **React 프론트** | `src/components/Widget.jsx` | ✕ 닫기 정상화, 위젯 위치 이동 추적 저장, WMI `—` 표시 |
| **React 프론트** | `src/components/ResourcePanel.jsx` | RAM 헤더 마이크로 버튼 배치, WMI `—` 표시, 패딩 동기화 |
| **CSS 스타일** | `src/App.css` | 핀 아이콘 무채색/컬러 토글, RAM 프로그레스 바 복구, 여백 및 간격 최적화 |
| **문서** | `update_0.9.0.md`, `review_0.9.0.md` | 릴리즈 노트 및 리뷰 리포트 생성 |

---

## 🧪 빌드 및 무결성 검증

* **Frontend**: `npm run build` 성공 (0 error, 0 warning)
* **Backend**: `cargo check` 성공 (0 error, 0 warning)
