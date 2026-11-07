# Code Review - sysinfo-app v0.1.0

**리뷰 일시**: 2026-01-19  
**앱 버전**: 0.1.0

---

## 📋 프로젝트 개요

Tauri + React 기반의 시스템 리소스 모니터링 데스크톱 애플리케이션입니다.  
CPU, 메모리, GPU(NVIDIA) 사용량을 실시간으로 표시합니다.

---

## ✅ 잘된 점

### 1. 아키텍처 설계
- **프론트엔드/백엔드 분리**가 깔끔하게 되어 있음
- **컴포넌트 구조**가 잘 분리됨 (`TitleBar`, `ResourcePanel`, `StatusBar`)
- **디자인 토큰** 시스템 (`App.css`의 CSS 변수) 사용으로 일관된 스타일링

### 2. 성능 최적화
- **Page Visibility API** 적용으로 백그라운드 폴링 방지 (`App.jsx:17`)
- **비동기 Tauri 명령어** 사용 (`lib.rs:7` - async fn)
- **useLayoutEffect**로 창 크기 조정 시 레이아웃 시프트 방지

### 3. UX/UI
- **다크모드 지원** (`prefers-color-scheme: dark` 미디어 쿼리)
- **접이식 섹션**으로 공간 효율적 활용
- **동적 창 크기 조정** - 콘텐츠에 맞게 자동 리사이즈

### 4. Rust 백엔드
- **NVML 통합**으로 GPU 정보 상세 제공 (온도, 전력 포함)
- **Mutex 기반 상태 관리**로 스레드 안전성 확보
- **깔끔한 에러 핸들링** (`ok()`, `unwrap_or_else`)

---

## ⚠️ 개선 제안

### 🔴 높은 우선순위

#### 1. CSS 중복 선언
```css
/* App.css:134-139 와 143-147 동일한 .resource-section 중복 */
.resource-section {
  background: var(--color-surface);
  padding: var(--spacing-md);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);  /* 첫 번째에만 있음 */
}
```
**문제**: 두 번째 선언에서 `box-shadow`가 누락되어 의도와 다르게 동작할 수 있음  
**해결**: 중복 제거 필요

#### 2. unused component - StatusBar.jsx
- `StatusBar` 컴포넌트가 `App.jsx`에서 import는 되어있지만 **렌더링되지 않음**
- `setIsResourcePanelOpen`, `isResourcePanelOpen` props가 정의되지 않음

```jsx
// App.jsx:6 - import는 되어있으나
import StatusBar from "./components/StatusBar";

// App.jsx:46-55 - 실제 렌더링에서 사용 안함
<div className="app">
  <TitleBar />
  <main className="main-content">
    <ResourcePanel systemResources={systemResources} />
  </main>
  {/* StatusBar 없음 */}
</div>
```

#### 3. GPU 하드코딩
```jsx
// ResourcePanel.jsx:114 - NVIDIA 뱃지 하드코딩
<span className="gpu-badge" style={{ marginLeft: '8px' }}>NVIDIA</span>
```
**문제**: AMD나 Intel GPU 지원 시 수정 필요  
**해결**: GPU 벤더 정보를 백엔드에서 동적으로 가져오기

---

### 🟡 중간 우선순위

#### 4. Mutex lock 개선
```rust
// system_info.rs:36
let mut sys = state.0.lock().unwrap();
```
**문제**: `unwrap()` 사용으로 panic 가능성  
**해결**: 
```rust
let mut sys = state.0.lock().map_err(|e| e.to_string())?;
```

#### 5. NVML 초기화 비용
```rust
// system_info.rs:73 - 매 호출마다 NVML 초기화
let nvml = Nvml::init().ok()?;
```
**문제**: 2초마다 NVML을 새로 초기화하는 것은 비효율적  
**해결**: `SystemState`에 NVML 인스턴스 캐싱

```rust
pub struct SystemState {
    pub sys: Mutex<System>,
    pub nvml: Option<Nvml>,  // 앱 시작 시 한번만 초기화
}
```

#### 6. 에러 상태 UI 없음
```jsx
// App.jsx:22-24 - 에러 시 로깅만 하고 UI 피드백 없음
} catch (e) {
  console.error("Failed to get system resources:", e);
}
```
**해결**: 에러 상태를 useState로 관리하고 사용자에게 표시

---

### 🟢 낮은 우선순위

#### 7. 폴링 주기 하드코딩
```jsx
// App.jsx:30
const interval = setInterval(fetchResources, 2000);
```
**제안**: 설정 가능하도록 변경

#### 8. 접근성 개선
- 프로그레스 바에 ARIA 속성 추가 필요
```jsx
<div className="resource-progress"
     role="progressbar"
     aria-valuenow={systemResources.cpu_usage_percent}
     aria-valuemin="0"
     aria-valuemax="100">
```

#### 9. 위젯 상태 동기화 문제
```jsx
// TitleBar.jsx:6 - 위젯 상태를 로컬로 관리
const [isWidgetOpen, setIsWidgetOpen] = useState(false);
```
**문제**: 위젯이 외부에서 닫히면 상태 불일치  
**해결**: 매번 `isVisible()` 확인 또는 이벤트 리스너 사용

---

## 📊 파일별 요약

| 파일 | 상태 | 주요 이슈 |
|------|------|----------|
| `App.jsx` | ✅ Good | 미사용 import (StatusBar) |
| `App.css` | ⚠️ Needs Fix | CSS 중복 규칙 |
| `TitleBar.jsx` | ✅ Good | 위젯 상태 동기화 개선 가능 |
| `ResourcePanel.jsx` | ⚠️ Needs Fix | NVIDIA 하드코딩 |
| `StatusBar.jsx` | ⚠️ Unused | 사용되지 않는 컴포넌트 |
| `lib.rs` | ✅ Good | - |
| `system_info.rs` | ⚠️ Needs Fix | NVML 캐싱, unwrap 개선 |

---

## 🎯 권장 실행 계획

1. **즉시 수정**: CSS 중복 제거, 미사용 import 제거
2. **다음 버전**: NVML 캐싱, GPU 벤더 동적 처리
3. **향후**: 에러 UI, 접근성, 폴링 설정

---

## 📈 전반적인 평가

**코드 품질**: ⭐⭐⭐⭐ (4/5)  
**구조 및 설계**: ⭐⭐⭐⭐⭐ (5/5)  
**성능 고려**: ⭐⭐⭐⭐ (4/5)  
**에러 핸들링**: ⭐⭐⭐ (3/5)  
**접근성**: ⭐⭐ (2/5)

전반적으로 잘 구조화된 Tauri 애플리케이션입니다!  
제안된 개선 사항을 적용하면 더욱 견고한 앱이 될 것입니다.
