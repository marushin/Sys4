# Code Review & Refactoring Report - sysinfo-app (Sys4) v0.9.2

**릴리즈 일시**: 2026-08-23  
**앱 버전**: v0.9.2  
**기술 스택**: Tauri v2, React 19, Rust, Windows Win32 API, Vite  

---

## 📋 v0.9.2 주요 개요

v0.9.2는 위젯 모드에서의 싱글 인스턴스 창 포커스 우선순위 정리와 네트워크 실시간 트래픽 산출 복구, 디스크 드라이브 마운트 경로 결합 및 프론트엔드 상시 렌더링 가드 안정화가 적용된 마이너 릴리즈입니다.

---

## ✅ 핵심 변경 및 개선 내역

### 1. 🛡️ 위젯 모드 중복 창 실행 방지
* `src-tauri/src/lib.rs`의 `single_instance` 핸들러에서 `widget` 창 가시성(`is_visible()`)을 우선 검사하여, 위젯 모드 실행 중 추가 실행 시 기본 메인 창이 중복 노출되는 문제를 제거하고 위젯에 포커스를 유지하도록 개선.

### 2. ⚡ 네트워크 및 디스크 실시간 데이터 수집 정상화
* **네트워크 트래픽 갱신**: `system_info.rs`에서 `refresh_list()` 대신 `refresh()`를 호출하여 인터페이스 재생성으로 인한 카운터 리셋을 방지하고 실시간 다운로드/업로드 속도 산출 정상화.
* **디스크 드라이브명 식별성 강화**: `disk.mount_point()`와 `disk.name()`을 결합하여 `C:\`, `D:\ (ccoonetwork)` 형태로 가독성 있게 표현.

### 3. 🎨 UI / UX 및 설정 동기화 안정성
* **네트워크/디스크 상시 렌더링**: `ResourcePanel.jsx`에서 가드 조건을 제거하여 접힘 상태에서도 카드 헤더(`네트워크 ▼`, `디스크 ▼`)를 상시 노출.
* **설정 동기화 보강**: `App.jsx`에 `isConfigLoaded` 플래그를 적용하여 초기 로드 시 `config.json` 덮어쓰기 방지.

---

## 🛠️ 버전 동기화 파일 목록

| 영역 | 파일 | 버전 |
|:---|:---|:---|
| **Frontend** | `package.json` | **v0.9.2** |
| **Backend** | `src-tauri/tauri.conf.json` | **v0.9.2** |
| **Backend** | `src-tauri/Cargo.toml` | **v0.9.2** |
| **문서** | `update_0.9.2.md`, `review_0.9.2.md` | **v0.9.2** |
