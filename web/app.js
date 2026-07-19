const app = document.querySelector('#app')
const toastRoot = document.querySelector('#toast-root')

const state = {
  user: null,
  minPasswordLength: 8,
  cleanup: [],
  renderedRoute: null,
  queue: { page: 1, mineOnly: false, studentId: '' },
  users: { page: 1, q: '' },
}

let routeNavigation = Promise.resolve()
let activeActionDialog = null

const routes = {
  submit: { label: '提交打印', icon: 'upload', admin: false },
  queue: { label: '打印队列', icon: 'list', admin: false },
  settings: { label: '个人设置', icon: 'settings', admin: false },
  users: { label: '用户管理', icon: 'users', admin: true },
  review: { label: '审核中心', icon: 'shield', admin: true },
  system: { label: '系统设置', icon: 'sliders', admin: true },
}

const iconPaths = {
  upload:
    '<path d="M12 16V4"></path><path d="m7 9 5-5 5 5"></path><path d="M5 20h14a2 2 0 0 0 2-2v-3"></path><path d="M3 15v3a2 2 0 0 0 2 2"></path>',
  list: '<path d="M9 6h11"></path><path d="M9 12h11"></path><path d="M9 18h11"></path><path d="M4 6h.01"></path><path d="M4 12h.01"></path><path d="M4 18h.01"></path>',
  settings:
    '<circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.83 2.83-.06-.06A1.7 1.7 0 0 0 15 19.4a1.7 1.7 0 0 0-1 .6 1.7 1.7 0 0 0-.4 1.1V21H9.6v-.1A1.7 1.7 0 0 0 8.5 19.4a1.7 1.7 0 0 0-1.88.34l-.06.06-2.83-2.83.06-.06A1.7 1.7 0 0 0 4.6 15a1.7 1.7 0 0 0-.6-1 1.7 1.7 0 0 0-1.1-.4H3V9.6h.1A1.7 1.7 0 0 0 4.6 8.5a1.7 1.7 0 0 0-.34-1.88l-.06-.06 2.83-2.83.06.06A1.7 1.7 0 0 0 9 4.6a1.7 1.7 0 0 0 1-.6 1.7 1.7 0 0 0 .4-1.1V3h4v.1A1.7 1.7 0 0 0 15.5 4.6a1.7 1.7 0 0 0 1.88-.34l.06-.06 2.83 2.83-.06.06A1.7 1.7 0 0 0 19.4 9c.35.28.56.7.6 1.1v3.8c-.04.45-.25.83-.6 1.1Z"></path>',
  users:
    '<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"></path><circle cx="9" cy="7" r="4"></circle><path d="M22 21v-2a4 4 0 0 0-3-3.87"></path><path d="M16 3.13a4 4 0 0 1 0 7.75"></path>',
  shield:
    '<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10Z"></path><path d="m9 12 2 2 4-4"></path>',
  sliders:
    '<path d="M4 21v-7"></path><path d="M4 10V3"></path><path d="M12 21v-9"></path><path d="M12 8V3"></path><path d="M20 21v-5"></path><path d="M20 12V3"></path><path d="M1 14h6"></path><path d="M9 8h6"></path><path d="M17 16h6"></path>',
  logout:
    '<path d="M10 17l5-5-5-5"></path><path d="M15 12H3"></path><path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4"></path>',
  send: '<path d="m22 2-7 20-4-9-9-4Z"></path><path d="M22 2 11 13"></path>',
  eye: '<path d="M2.25 12s3.5-6 9.75-6 9.75 6 9.75 6-3.5 6-9.75 6-9.75-6-9.75-6Z"></path><circle cx="12" cy="12" r="2.75"></circle>',
  x: '<path d="m6 6 12 12"></path><path d="m18 6-12 12"></path>',
  save: '<path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2Z"></path><path d="M17 21v-8H7v8"></path><path d="M7 3v5h8"></path>',
  search: '<circle cx="11" cy="11" r="7"></circle><path d="m20 20-4-4"></path>',
  plus: '<path d="M12 5v14"></path><path d="M5 12h14"></path>',
  download: '<path d="M12 3v12"></path><path d="m7 10 5 5 5-5"></path><path d="M5 21h14"></path>',
  pause: '<path d="M8 5v14"></path><path d="M16 5v14"></path>',
  play: '<path d="m6 3 14 9-14 9Z"></path>',
  check: '<path d="m5 12 4 4L19 6"></path>',
  key: '<circle cx="8" cy="15" r="4"></circle><path d="m11 12 9-9"></path><path d="m18 5 2 2"></path><path d="m15 8 2 2"></path>',
  ban: '<circle cx="12" cy="12" r="9"></circle><path d="m5.6 5.6 12.8 12.8"></path>',
  trash: '<path d="M3 6h18"></path><path d="M8 6V4h8v2"></path><path d="m19 6-1 15H6L5 6"></path><path d="M10 11v5"></path><path d="M14 11v5"></path>',
  left: '<path d="m15 18-6-6 6-6"></path>',
  right: '<path d="m9 18 6-6-6-6"></path>',
}

function icon(name, size = 18) {
  return `<svg class="ui-icon" width="${size}" height="${size}" viewBox="0 0 24 24" aria-hidden="true">${iconPaths[name] || ''}</svg>`
}

function escapeHtml(value = '') {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#039;')
}

function rawFormValue(form, name) {
  return String(new FormData(form).get(name) || '')
}

function formValue(form, name) {
  return rawFormValue(form, name).trim()
}

function localApiUrl(value) {
  try {
    const url = new URL(String(value), location.origin)
    if (url.origin !== location.origin || !url.pathname.startsWith('/api/')) return null
    return `${url.pathname}${url.search}${url.hash}`
  } catch {
    return null
  }
}

function passwordField(label, name, autocomplete, minLength = null) {
  const minLengthAttribute = minLength === null ? '' : ` minlength="${minLength}"`
  return `
    <label class="password-field">
      <span>${label}</span>
      <span class="password-control">
        <input name="${name}" type="password"${minLengthAttribute} autocomplete="${autocomplete}" required />
        <button class="password-toggle" type="button" aria-label="显示密码" aria-pressed="false" title="显示密码">
          <svg class="eye-icon eye-open" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M2.25 12s3.5-6 9.75-6 9.75 6 9.75 6-3.5 6-9.75 6-9.75-6-9.75-6Z"></path>
            <circle cx="12" cy="12" r="2.75"></circle>
          </svg>
          <svg class="eye-icon eye-closed" viewBox="0 0 24 24" aria-hidden="true">
            <path d="m3 3 18 18"></path>
            <path d="M10.6 6.1Q11.3 6 12 6c6.25 0 9.75 6 9.75 6a17 17 0 0 1-3.05 3.65M6.25 7.35C3.65 9.2 2.25 12 2.25 12S5.75 18 12 18q1.65 0 3.05-.55"></path>
            <path d="M9.9 9.9a3 3 0 0 0 4.2 4.2"></path>
          </svg>
        </button>
      </span>
    </label>`
}

function setupPasswordFields(root = document) {
  root.querySelectorAll('.password-control').forEach((control) => {
    const input = control.querySelector('input')
    const toggle = control.querySelector('.password-toggle')
    if (!input || !toggle) return

    const update = () => control.classList.toggle('has-value', input.value.length > 0)
    input.addEventListener('input', update)
    input.addEventListener('focus', update)
    input.addEventListener('change', update)
    toggle.addEventListener('click', () => {
      const showPassword = input.type === 'password'
      input.type = showPassword ? 'text' : 'password'
      toggle.setAttribute('aria-pressed', String(showPassword))
      toggle.setAttribute('aria-label', showPassword ? '隐藏密码' : '显示密码')
      toggle.title = showPassword ? '隐藏密码' : '显示密码'
      input.focus()
    })
    update()
  })
}

function routeName() {
  return location.hash.replace(/^#\/?/, '').split('?')[0] || 'submit'
}

function replaceRouteHash(route) {
  history.replaceState(null, '', `${location.pathname}${location.search}#/${route}`)
}

function notify(message, tone = 'info') {
  const item = document.createElement('div')
  item.className = `notification ${tone}`
  item.textContent = message
  toastRoot.append(item)
  const dismiss = () => {
    if (!item.isConnected || item.classList.contains('closing')) return
    item.classList.add('closing')
    item.addEventListener('animationend', () => item.remove(), { once: true })
    setTimeout(() => item.remove(), 500)
  }
  setTimeout(dismiss, 4200)
}

function openActionDialog({
  title,
  message = '',
  confirmText = '确认',
  danger = false,
  inputLabel = '',
  inputValue = '',
  inputRequired = false,
  onConfirm,
}) {
  activeActionDialog?.()
  const returnFocus = document.activeElement
  const backdrop = document.createElement('div')
  backdrop.className = 'dialog-backdrop'
  backdrop.innerHTML = `
    <form class="confirm-dialog" role="dialog" aria-modal="true" aria-label="${escapeHtml(title)}">
      <header>
        <strong>${escapeHtml(title)}</strong>
        <button class="icon-button dialog-close" type="button" title="关闭" aria-label="关闭">${icon('x')}</button>
      </header>
      ${message ? `<p>${escapeHtml(message)}</p>` : ''}
      ${
        inputLabel
          ? `<label class="dialog-input">${escapeHtml(inputLabel)}<input class="dialog-value" value="${escapeHtml(inputValue)}" autocomplete="off" ${inputRequired ? 'required' : ''} /></label>`
          : ''
      }
      <footer>
        <button class="ghost-button dialog-cancel" type="button">取消</button>
        <button class="primary-button dialog-confirm ${danger ? 'dialog-danger-button' : ''}" type="submit">${escapeHtml(confirmText)}</button>
      </footer>
    </form>`
  const form = backdrop.querySelector('form')
  const closeButton = backdrop.querySelector('.dialog-close')
  const cancelButton = backdrop.querySelector('.dialog-cancel')
  const confirmButton = backdrop.querySelector('.dialog-confirm')
  const input = backdrop.querySelector('.dialog-value')
  let busy = false
  let closed = false

  const close = (force = false) => {
    if (closed || (busy && !force)) return
    closed = true
    window.removeEventListener('keydown', closeOnEscape)
    backdrop.remove()
    if (activeActionDialog === close) activeActionDialog = null
    if (returnFocus?.isConnected) returnFocus.focus()
  }
  const closeOnEscape = (event) => {
    if (event.key === 'Escape') close()
  }
  const setBusy = (nextBusy) => {
    busy = nextBusy
    closeButton.disabled = nextBusy
    cancelButton.disabled = nextBusy
    confirmButton.disabled = nextBusy
    confirmButton.textContent = nextBusy ? '处理中…' : confirmText
    if (input) input.disabled = nextBusy
  }

  backdrop.addEventListener('click', (event) => {
    if (event.target === backdrop) close()
  })
  closeButton.addEventListener('click', () => close())
  cancelButton.addEventListener('click', () => close())
  window.addEventListener('keydown', closeOnEscape)
  form.addEventListener('submit', async (event) => {
    event.preventDefault()
    if (busy) return
    const value = input?.value.trim() || ''
    if (inputRequired && !value) {
      input.focus()
      return
    }
    setBusy(true)
    try {
      await onConfirm(value)
      close(true)
    } catch (error) {
      notify(error.message, 'error')
      setBusy(false)
    }
  })

  app.append(backdrop)
  activeActionDialog = close
  state.cleanup.push(() => close(true))
  if (input) input.focus()
  else confirmButton.focus()
}

function openImportUsersDialog(onImported) {
  activeActionDialog?.()
  const returnFocus = document.activeElement
  const backdrop = document.createElement('div')
  backdrop.className = 'dialog-backdrop'
  backdrop.innerHTML = `
    <form class="confirm-dialog batch-import-dialog" role="dialog" aria-modal="true" aria-label="批量导入用户">
      <header>
        <strong>批量导入用户</strong>
        <button class="icon-button dialog-close" type="button" title="关闭" aria-label="关闭">${icon('x')}</button>
      </header>
      <label class="dialog-input">
        学号
        <textarea class="batch-student-ids" rows="9" placeholder="每行输入一个学号" autocomplete="off"></textarea>
      </label>
      <div class="batch-format-note">
        <p>也可以选择 TXT 或 Excel 文件：</p>
        <ul>
          <li>TXT 文件每行一个学号</li>
          <li>Excel 文件每个单元格一个学号</li>
        </ul>
      </div>
      <label class="ghost-button batch-file-button">
        ${icon('upload')}
        <span class="batch-file-name">选择 TXT 或 Excel 文件</span>
        <input class="batch-file-input" type="file" accept=".txt,.xlsx,.xls,.xlsm" hidden />
      </label>
      <footer>
        <button class="ghost-button dialog-cancel" type="button">取消</button>
        <button class="primary-button dialog-confirm" type="submit">批量导入</button>
      </footer>
    </form>`

  const form = backdrop.querySelector('form')
  const closeButton = backdrop.querySelector('.dialog-close')
  const cancelButton = backdrop.querySelector('.dialog-cancel')
  const confirmButton = backdrop.querySelector('.dialog-confirm')
  const textarea = backdrop.querySelector('.batch-student-ids')
  const fileInput = backdrop.querySelector('.batch-file-input')
  const fileName = backdrop.querySelector('.batch-file-name')
  let selectedFile = null
  let busy = false
  let closed = false

  const close = (force = false) => {
    if (closed || (busy && !force)) return
    closed = true
    window.removeEventListener('keydown', closeOnEscape)
    backdrop.remove()
    if (activeActionDialog === close) activeActionDialog = null
    if (returnFocus?.isConnected) returnFocus.focus()
  }
  const closeOnEscape = (event) => {
    if (event.key === 'Escape') close()
  }
  const setBusy = (nextBusy) => {
    busy = nextBusy
    closeButton.disabled = nextBusy
    cancelButton.disabled = nextBusy
    confirmButton.disabled = nextBusy
    textarea.disabled = nextBusy
    fileInput.disabled = nextBusy
    confirmButton.textContent = nextBusy ? '导入中…' : '批量导入'
  }

  backdrop.addEventListener('click', (event) => {
    if (event.target === backdrop) close()
  })
  closeButton.addEventListener('click', () => close())
  cancelButton.addEventListener('click', () => close())
  window.addEventListener('keydown', closeOnEscape)
  fileInput.addEventListener('change', () => {
    selectedFile = fileInput.files[0] || null
    if (!selectedFile) return
    textarea.value = ''
    fileName.textContent = selectedFile.name
  })
  textarea.addEventListener('input', () => {
    if (!selectedFile) return
    selectedFile = null
    fileInput.value = ''
    fileName.textContent = '选择 TXT 或 Excel 文件'
  })
  form.addEventListener('submit', async (event) => {
    event.preventDefault()
    if (busy) return
    const studentIds = textarea.value.trim()
    if (!selectedFile && !studentIds) {
      notify('请输入学号或选择导入文件', 'error')
      textarea.focus()
      return
    }

    const body = new FormData()
    if (selectedFile) {
      body.append('file', selectedFile)
    } else {
      body.append('file', new Blob([studentIds], { type: 'text/plain;charset=utf-8' }), 'users.txt')
    }
    setBusy(true)
    try {
      const result = await api('/admin/users/import', { method: 'POST', body })
      notify(`创建 ${result.created.length} 个，跳过 ${result.skipped.length} 个`, 'success')
      await onImported()
      close(true)
    } catch (error) {
      notify(error.message, 'error')
      setBusy(false)
    }
  })

  app.append(backdrop)
  activeActionDialog = close
  state.cleanup.push(() => close(true))
  textarea.focus()
}

function setSession(response) {
  state.user = response.user
  state.minPasswordLength =
    Number.isInteger(response.min_password_length) && response.min_password_length > 0
      ? response.min_password_length
      : 8
}

async function api(path, options = {}) {
  const headers = new Headers(options.headers)
  let body = options.body
  if (body && !(body instanceof FormData) && typeof body !== 'string') {
    headers.set('content-type', 'application/json')
    body = JSON.stringify(body)
  }
  let response
  try {
    response = await fetch(`/api${path}`, {
      credentials: 'same-origin',
      ...options,
      headers,
      body,
    })
  } catch {
    throw new Error('无法连接服务器，请检查网络连接或确认程序正在运行')
  }
  if (response.status === 401) {
    state.user = null
    renderLogin()
  }
  if (!response.ok) {
    let message = `请求失败（${response.status}）`
    try {
      const data = await response.json()
      message = data.error || message
    } catch {}
    const error = new Error(message)
    error.status = response.status
    throw error
  }
  const type = response.headers.get('content-type') || ''
  return type.includes('application/json') ? response.json() : response.text()
}

function clearRuntime() {
  state.cleanup.splice(0).forEach((cleanup) => cleanup())
}

function shell(content) {
  clearRuntime()
  const shellKey = [
    state.user.id,
    state.user.student_id,
    state.user.role,
    state.user.must_change_password,
  ].join(':')
  const existingView = document.querySelector('#view')
  if (existingView && app.dataset.shellKey === shellKey) {
    existingView.classList.remove(
      'page-leave-active',
      'page-leave-to',
      'page-enter-active',
      'page-enter-from',
    )
    existingView.innerHTML = content
    updateNavHighlight()
    return
  }
  const current = routeName()
  const mustChange = state.user.must_change_password
  const nav = Object.entries(routes)
    .filter(([, item]) => !item.admin && (!mustChange || item.label === '个人设置'))
    .map(
      ([name, item]) =>
        `<a class="nav-link ${current === name ? 'router-link-active' : ''}" href="#/${name}">${icon(item.icon)}<span>${item.label}</span></a>`,
    )
    .join('')
  const adminNav = Object.entries(routes)
    .filter(([, item]) => item.admin && state.user.role === 'admin' && !mustChange)
    .map(
      ([name, item]) =>
        `<a class="nav-link ${current === name ? 'router-link-active' : ''}" href="#/${name}">${icon(item.icon)}<span>${item.label}</span></a>`,
    )
    .join('')
  app.className = ''
  app.dataset.shellKey = shellKey
  app.innerHTML = `
    <div class="app-shell">
      <aside class="sidebar">
        <div class="brand">
          <img class="brand-logo" src="/logo.svg" alt="" />
          <div><strong>ACM Print</strong><span>${escapeHtml(state.user.student_id)}</span></div>
        </div>
        <nav class="nav-group">${nav}</nav>
        ${adminNav ? `<nav class="nav-group admin-nav">${adminNav}</nav>` : ''}
        <button id="logout" class="ghost-button logout-button" type="button">${icon('logout')}<span>退出登录</span></button>
      </aside>
      <main class="main-panel"><div id="view" class="route-frame">${content}</div></main>
    </div>`
  document.querySelector('#logout').addEventListener('click', logout)
}

async function restoreSession() {
  try {
    setSession(await api('/auth/me'))
    if (state.user.must_change_password) replaceRouteHash('settings')
    await renderRoute({ animate: true })
  } catch (error) {
    if (!state.user) renderLogin()
    if (error.status !== 401) notify(error.message, 'error')
  }
}

function renderLogin() {
  if (state.renderedRoute === 'login' && app.querySelector('.login-screen')) return
  clearRuntime()
  state.renderedRoute = 'login'
  app.className = ''
  delete app.dataset.shellKey
  app.innerHTML = `
    <main class="login-screen">
      <section class="login-panel">
        <div class="login-brand">
          <img class="login-logo" src="/logo.svg" alt="" />
          <h1>ACM 实验室自助打印平台</h1>
        </div>
        <form id="login-form" class="form-grid">
          <label>学号<input name="student_id" autocomplete="username" required autofocus /></label>
          ${passwordField('密码', 'password', 'current-password')}
          <button class="primary-button" type="submit">登录</button>
        </form>
      </section>
    </main>`
  setupPasswordFields(app)
  document.querySelector('#login-form').addEventListener('submit', async (event) => {
    event.preventDefault()
    const button = event.currentTarget.querySelector('button[type="submit"]')
    button.disabled = true
    try {
      setSession(
        await api('/auth/login', {
          method: 'POST',
          body: {
            student_id: formValue(event.currentTarget, 'student_id'),
            password: rawFormValue(event.currentTarget, 'password'),
          },
        }),
      )
      replaceRouteHash(state.user.must_change_password ? 'settings' : 'submit')
      await renderRoute({ animate: true })
    } catch (error) {
      notify(error.message, 'error')
    } finally {
      button.disabled = false
    }
  })
}

async function logout() {
  try {
    await api('/auth/logout', { method: 'POST' })
  } catch {}
  state.user = null
  history.replaceState(null, '', location.pathname)
  renderLogin()
}

function finishRouteTransition(view, activeClass, endClass) {
  view.classList.remove(activeClass, endClass)
}

function updateNavHighlight(route = routeName()) {
  document.querySelectorAll('.nav-link').forEach((link) => {
    link.classList.toggle('router-link-active', link.getAttribute('href') === `#/${route}`)
  })
}

function animateRouteLeave(view) {
  return new Promise((resolve) => {
    let finished = false
    const finish = () => {
      if (finished) return
      finished = true
      clearTimeout(fallback)
      view.removeEventListener('transitionend', onTransitionEnd)
      resolve()
    }
    const onTransitionEnd = (event) => {
      if (event.target === view && event.propertyName === 'transform') finish()
    }
    const fallback = setTimeout(finish, 480)
    view.addEventListener('transitionend', onTransitionEnd)
    view.classList.add('page-leave-active')
    requestAnimationFrame(() => {
      if (!view.isConnected) return finish()
      view.classList.add('page-leave-to')
    })
  })
}

function animateRouteEnter(view) {
  view.classList.add('page-enter-active', 'page-enter-from')
  const finish = (event) => {
    if (event && (event.target !== view || event.propertyName !== 'transform')) return
    clearTimeout(fallback)
    view.removeEventListener('transitionend', finish)
    finishRouteTransition(view, 'page-enter-active', 'page-enter-from')
  }
  const fallback = setTimeout(() => finish(), 480)
  view.addEventListener('transitionend', finish)
  requestAnimationFrame(() =>
    requestAnimationFrame(() => {
      if (view.isConnected) view.classList.remove('page-enter-from')
      else finish()
    }),
  )
}

async function renderRoute({ animate = false } = {}) {
  if (!state.user) return renderLogin()
  let route = routeName()
  if (!routes[route]) {
    route = 'submit'
    replaceRouteHash(route)
  }
  if (state.user.must_change_password && route !== 'settings') {
    route = 'settings'
    replaceRouteHash(route)
  }
  if (routes[route].admin && state.user.role !== 'admin') {
    route = 'submit'
    replaceRouteHash(route)
  }
  shell('<section class="page page-loading-shell"></section>')
  try {
    await {
      submit: renderSubmit,
      queue: renderQueue,
      settings: renderSettings,
      users: renderUsers,
      review: renderReview,
      system: renderSystem,
    }[route]()
  } catch (error) {
    const view = document.querySelector('#view')
    if (view) view.innerHTML = '<section class="page page-loading-shell"></section>'
    notify(error.message, 'error')
  }
  const view = document.querySelector('#view')
  if (!state.user || !view) return
  state.renderedRoute = route
  if (animate) animateRouteEnter(view)
}

async function transitionRoute() {
  if (!state.user) return renderLogin()
  updateNavHighlight()
  if (state.renderedRoute === routeName()) return
  const view = document.querySelector('#view')
  if (view && view.childElementCount > 0) await animateRouteLeave(view)
  await renderRoute({ animate: true })
}

function pageHeader(title, description, actions = '') {
  return `<header class="page-header"><div><h1>${title}</h1>${description ? `<p>${description}</p>` : ''}</div>${actions}</header>`
}

function parseCustomPageRange(value, totalPages) {
  const tokens = String(value)
    .trim()
    .split(/[\s,，、]+/u)
    .filter(Boolean)
  if (tokens.length === 0) {
    return { pages: [], normalized: '', error: '' }
  }

  const pages = new Set()
  for (const token of tokens) {
    const match = token.match(/^(\d+)(?:-(\d+))?$/u)
    if (!match) {
      return {
        pages: [],
        normalized: '',
        error: `页码格式“${token}”无效，请使用数字或起止页（如 2-5）`,
      }
    }
    const start = Number(match[1])
    const end = Number(match[2] || match[1])
    if (
      !Number.isSafeInteger(start) ||
      !Number.isSafeInteger(end) ||
      start < 1 ||
      end > totalPages
    ) {
      return {
        pages: [],
        normalized: '',
        error: `页码范围“${token}”超出有效范围 1-${totalPages}`,
      }
    }
    if (start > end) {
      return {
        pages: [],
        normalized: '',
        error: `页码范围“${token}”的起始页不能大于结束页`,
      }
    }
    for (let page = start; page <= end; page += 1) {
      if (pages.has(page)) {
        return {
          pages: [],
          normalized: '',
          error: `自定义页码中重复指定了第 ${page} 页`,
        }
      }
      pages.add(page)
    }
  }

  const sortedPages = [...pages].sort((left, right) => left - right)
  const ranges = []
  let start = sortedPages[0]
  let end = start
  for (const page of sortedPages.slice(1)) {
    if (page === end + 1) {
      end = page
      continue
    }
    ranges.push(start === end ? String(start) : `${start}-${end}`)
    start = page
    end = page
  }
  ranges.push(start === end ? String(start) : `${start}-${end}`)
  return { pages: sortedPages, normalized: ranges.join(','), error: '' }
}

function selectedPages(file) {
  if (file.status !== 'ready') return 0
  if (file.odd_even === 'odd') return Math.ceil(file.page_count / 2)
  if (file.odd_even === 'even') return Math.floor(file.page_count / 2)
  if (file.odd_even === 'custom') {
    return parseCustomPageRange(file.custom_range, file.page_count).pages.length
  }
  return file.page_count
}

function selectedPagesSummary(file) {
  if (file.status === 'uploading') return '正在上传并生成预览…'
  if (file.status === 'failed') return `处理失败：${file.error}`
  if (file.odd_even === 'all') return `${file.page_count} 页`
  if (file.odd_even === 'custom') {
    const selection = parseCustomPageRange(file.custom_range, file.page_count)
    return selection.error
      ? `${file.page_count} 页 · 本次打印 — 页`
      : `${file.page_count} 页 · 本次打印 ${selection.pages.length} 页`
  }
  return `${file.page_count} 页 · 本次打印 ${selectedPages(file)} 页`
}

function pageSelectionLabel(selection) {
  if (selection === 'odd') return '奇数页'
  if (selection === 'even') return '偶数页'
  if (selection.startsWith('custom:')) return `自定义页：${selection.slice('custom:'.length)}`
  return '全部页'
}

async function renderSubmit() {
  const [quota, uploadsResponse, stats] = await Promise.all([
    api('/user/quota'),
    api('/print/uploads'),
    api('/user/submit-stats'),
  ])
  const uploads = uploadsResponse.files.map((file) => ({
    ...file,
    client_id: file.temp_id,
    status: 'ready',
    odd_even: 'all',
    custom_range: '',
  }))
  const quotaTransitionMs = 220
  let quotaAnimationFrame = null
  let previewModal = null
  let previewFileId = null
  let previewReturnFocus = null
  let previewCloseTimer = null
  let submitting = false
  let localUploadSequence = 0

  const projectedPages = () =>
    uploads.reduce((sum, file) => sum + selectedPages(file), 0)

  const quotaWidths = (projected) => ({
    green:
      quota.limit <= 0
        ? 0
        : (Math.max(0, quota.remaining - projected) / quota.limit) * 100,
    pending:
      quota.limit <= 0
        ? 0
        : (Math.min(projected, quota.remaining) / quota.limit) * 100,
  })

  const requiresApproval = (projected) => projected > quota.remaining

  const quotaLegendMarkup = (projected) =>
    projected
      ? `<span class="quota-projected-label"><i class="pending-dot"></i>本次打印 ${projected} 页${requiresApproval(projected) ? '<em class="review-required">需审批</em>' : ''}</span>`
      : ''

  const uploadCardMarkup = (file) => {
    const ready = file.status === 'ready'
    const uploading = file.status === 'uploading'
    const summary = selectedPagesSummary(file)
    const previewContent = uploading ? '<span class="upload-spinner" aria-hidden="true"></span>' : icon(ready ? 'eye' : 'x')
    return `
    <article class="upload-card upload-${file.status}" data-id="${escapeHtml(file.client_id)}">
      <div class="upload-card-heading">
        <button class="icon-button preview-upload ${uploading ? 'preview-loading' : ready ? '' : 'preview-failed'}" title="${ready ? '预览' : uploading ? '正在生成预览' : '文件处理失败'}" aria-label="${ready ? '预览' : uploading ? '正在生成预览' : '文件处理失败'}" type="button" ${ready ? '' : 'disabled'}>${previewContent}</button>
        <div class="file-details"><strong title="${escapeHtml(file.original_name)}">${escapeHtml(file.original_name)}</strong><span title="${escapeHtml(summary)}">${escapeHtml(summary)}</span></div>
        <button class="icon-button remove-button remove-upload" title="移出" aria-label="移出" type="button" ${uploading ? 'disabled' : ''}>${icon('x')}</button>
      </div>
      ${
        ready || uploading
          ? `<div class="page-range-section">
        <div class="page-range-control" data-selection="${file.odd_even}">
          <button type="button" data-range="all" class="${file.odd_even === 'all' ? 'active' : ''}" ${uploading ? 'disabled' : ''}>全部页</button>
          <button type="button" data-range="odd" class="${file.odd_even === 'odd' ? 'active' : ''}" ${uploading ? 'disabled' : ''}>奇数页</button>
          <button type="button" data-range="even" ${uploading || file.page_count < 2 ? 'disabled' : ''} class="${file.odd_even === 'even' ? 'active' : ''}">偶数页</button>
          <button type="button" data-range="custom" class="${file.odd_even === 'custom' ? 'active' : ''}" ${uploading ? 'disabled' : ''}>自定义</button>
        </div>
        ${
          ready
            ? `<div class="custom-page-editor ${file.odd_even === 'custom' ? 'open' : ''}" aria-hidden="${file.odd_even === 'custom' ? 'false' : 'true'}">
          <div class="custom-page-editor-inner">
            <input class="custom-page-input" type="text" inputmode="numeric" value="${escapeHtml(file.custom_range)}" placeholder="例如：1-3,5,7" aria-label="自定义打印页码" ${file.odd_even === 'custom' ? '' : 'disabled'} />
            <div class="custom-page-error-wrap ${file.odd_even === 'custom' && parseCustomPageRange(file.custom_range, file.page_count).error ? 'visible' : ''}">
              <p class="custom-page-error" role="alert">${file.odd_even === 'custom' ? escapeHtml(parseCustomPageRange(file.custom_range, file.page_count).error) : ''}</p>
            </div>
          </div>
        </div>`
            : ''
        }
      </div>`
          : ''
      }
    </article>`
  }

  const setQuotaSegments = (green, pending) => {
    const greenSegment = document.querySelector('.quota-green-segment')
    const pendingSegment = document.querySelector('.quota-pending-segment')
    if (!greenSegment || !pendingSegment) return
    greenSegment.setAttribute('width', String(green))
    pendingSegment.setAttribute('x', String(green))
    pendingSegment.setAttribute('width', String(pending))
  }

  const animateQuotaSegments = (greenTarget, pendingTarget) => {
    const greenSegment = document.querySelector('.quota-green-segment')
    const pendingSegment = document.querySelector('.quota-pending-segment')
    if (!greenSegment || !pendingSegment) return

    if (quotaAnimationFrame !== null) cancelAnimationFrame(quotaAnimationFrame)
    const greenStart = Number(greenSegment.getAttribute('width')) || 0
    const pendingStart = Number(pendingSegment.getAttribute('width')) || 0
    const startedAt = performance.now()

    const tick = (now) => {
      const elapsed = Math.min((now - startedAt) / quotaTransitionMs, 1)
      const eased = 1 - Math.pow(1 - elapsed, 3)
      setQuotaSegments(
        greenStart + (greenTarget - greenStart) * eased,
        pendingStart + (pendingTarget - pendingStart) * eased,
      )
      if (elapsed < 1) {
        quotaAnimationFrame = requestAnimationFrame(tick)
      } else {
        quotaAnimationFrame = null
      }
    }
    quotaAnimationFrame = requestAnimationFrame(tick)
  }

  const updateSubmitSummary = (animate = true) => {
    const projected = projectedPages()
    const readyUploads = uploads.filter((file) => file.status === 'ready')
    const hasPendingUpload = uploads.some((file) => file.status === 'uploading')
    const hasInvalidSelection = uploads.some(
      (file) =>
        file.status === 'ready' &&
        file.odd_even === 'custom' &&
        (Boolean(parseCustomPageRange(file.custom_range, file.page_count).error) ||
          selectedPages(file) === 0),
    )
    const widths = quotaWidths(projected)
    const quotaCard = document.querySelector('.quota-progress-card')
    quotaCard?.classList.toggle('danger', requiresApproval(projected))
    const quotaTrack = document.querySelector('.quota-track')
    quotaTrack?.setAttribute('aria-valuetext', `剩余 ${quota.remaining} 页，本次打印 ${projected} 页`)
    const legend = document.querySelector('.quota-legend')
    if (legend) legend.innerHTML = quotaLegendMarkup(projected)
    const submitButton = document.querySelector('#submit-files')
    if (submitButton) {
      submitButton.disabled =
        submitting || readyUploads.length === 0 || hasPendingUpload || hasInvalidSelection
      submitButton.querySelector('span').textContent = requiresApproval(projected)
        ? '提交审批'
        : '提交打印'
    }
    if (animate) animateQuotaSegments(widths.green, widths.pending)
    else setQuotaSegments(widths.green, widths.pending)
  }

  const updateUploadCard = (card, file) => {
    const rangeControl = card.querySelector('.page-range-control')
    rangeControl.dataset.selection = file.odd_even
    rangeControl.querySelectorAll('[data-range]').forEach((button) => {
      button.classList.toggle('active', button.dataset.range === file.odd_even)
    })
    card.querySelector('.file-details span').textContent = selectedPagesSummary(file)
    const editor = card.querySelector('.custom-page-editor')
    const input = card.querySelector('.custom-page-input')
    const error = card.querySelector('.custom-page-error')
    const errorWrap = card.querySelector('.custom-page-error-wrap')
    const selection = parseCustomPageRange(file.custom_range, file.page_count)
    const customIsOpen = file.odd_even === 'custom'
    editor.classList.toggle('open', customIsOpen)
    editor.setAttribute('aria-hidden', String(!customIsOpen))
    input.disabled = !customIsOpen
    input.setAttribute(
      'aria-invalid',
      String(customIsOpen && Boolean(selection.error)),
    )
    error.textContent = customIsOpen ? selection.error : ''
    errorWrap.classList.toggle('visible', customIsOpen && Boolean(selection.error))
  }

  const closePreview = (immediate = false) => {
    if (!previewModal) return
    const modal = previewModal
    if (modal.classList.contains('closing') && !immediate) return
    const finish = () => {
      clearTimeout(previewCloseTimer)
      modal.remove()
      if (previewModal === modal) {
        previewModal = null
        previewFileId = null
        if (previewReturnFocus?.isConnected) previewReturnFocus.focus()
        previewReturnFocus = null
      }
    }
    if (immediate) return finish()
    modal.classList.add('closing')
    modal.querySelector('.preview-dialog').addEventListener('animationend', finish, { once: true })
    previewCloseTimer = setTimeout(finish, 320)
  }

  const showPreview = (file, trigger) => {
    const sourceUrl = localApiUrl(file.preview_url)
    if (!sourceUrl) return notify('预览地址无效', 'error')
    closePreview(true)
    previewReturnFocus = trigger
    previewFileId = String(file.temp_id)
    const previewUrl = sourceUrl.includes('#') ? sourceUrl : `${sourceUrl}#zoom=100`
    const modal = document.createElement('div')
    modal.className = 'preview-modal'
    modal.setAttribute('role', 'dialog')
    modal.setAttribute('aria-modal', 'true')
    modal.setAttribute('aria-labelledby', 'preview-title')
    modal.innerHTML = `
      <section class="preview-dialog">
        <header>
          <strong id="preview-title" title="${escapeHtml(file.original_name)}">${escapeHtml(file.original_name)}</strong>
          <button class="icon-button close-preview" type="button" title="关闭预览" aria-label="关闭预览">${icon('x')}</button>
        </header>
        <iframe src="${escapeHtml(previewUrl)}" title="PDF 预览"></iframe>
      </section>`
    modal.addEventListener('click', (event) => {
      if (event.target === modal) closePreview()
    })
    modal.querySelector('.close-preview').addEventListener('click', () => closePreview())
    document.querySelector('#view').append(modal)
    previewModal = modal
    modal.querySelector('.close-preview').focus()
  }

  const closePreviewOnEscape = (event) => {
    if (event.key === 'Escape') closePreview()
  }
  window.addEventListener('keydown', closePreviewOnEscape)

  const bindUploadCard = (card) => {
    const file = uploads.find((item) => String(item.client_id) === card.dataset.id)
    if (!file) return
    if (file.status === 'uploading') return
    if (file.status === 'ready') {
      card
        .querySelector('.preview-upload')
        .addEventListener('click', (event) => showPreview(file, event.currentTarget))
    }
    card.querySelector('.remove-upload').addEventListener('click', async () => {
      try {
        if (file.status === 'ready') {
          await api(`/print/uploads/${file.temp_id}`, { method: 'DELETE' })
          if (previewFileId === String(file.temp_id)) closePreview()
        }
        uploads.splice(uploads.indexOf(file), 1)
        card.remove()
        const uploadList = document.querySelector('.upload-list')
        if (uploadList && uploads.length === 0) {
          uploadList.innerHTML = '<p class="empty-upload-list">尚未添加文件</p>'
        }
        updateSubmitSummary()
      } catch (error) {
        notify(error.message, 'error')
      }
    })
    if (file.status !== 'ready') return
    card.querySelectorAll('[data-range]').forEach((button) =>
      button.addEventListener('click', () => {
        if (file.odd_even === button.dataset.range) {
          if (file.odd_even === 'custom') card.querySelector('.custom-page-input').focus()
          return
        }
        file.odd_even = button.dataset.range
        updateUploadCard(card, file)
        updateSubmitSummary()
        if (file.odd_even === 'custom') card.querySelector('.custom-page-input').focus()
      }),
    )
    const customInput = card.querySelector('.custom-page-input')
    customInput.addEventListener('input', () => {
      file.custom_range = customInput.value
      updateUploadCard(card, file)
      updateSubmitSummary()
    })
    customInput.addEventListener('blur', () => {
      const selection = parseCustomPageRange(file.custom_range, file.page_count)
      if (selection.error) return
      file.custom_range = selection.normalized
      customInput.value = selection.normalized
      updateUploadCard(card, file)
    })
  }

  const appendUploadCards = (files) => {
    const uploadList = document.querySelector('.upload-list')
    if (!uploadList || files.length === 0) return
    uploadList.querySelector('.empty-upload-list')?.remove()
    files.forEach((file) => {
      const template = document.createElement('template')
      template.innerHTML = uploadCardMarkup(file).trim()
      const card = template.content.firstElementChild
      uploadList.append(card)
      bindUploadCard(card)
    })
    updateSubmitSummary()
    requestAnimationFrame(() => {
      uploadList.scrollTo({ top: uploadList.scrollHeight, behavior: 'smooth' })
    })
  }

  const replaceUploadCard = (file) => {
    const currentCard = [...document.querySelectorAll('.upload-card')].find(
      (card) => card.dataset.id === String(file.client_id),
    )
    if (!currentCard) return
    const template = document.createElement('template')
    template.innerHTML = uploadCardMarkup(file).trim()
    const nextCard = template.content.firstElementChild
    currentCard.replaceWith(nextCard)
    bindUploadCard(nextCard)
    updateSubmitSummary()
  }

  const renderPage = () => {
    const projected = uploads.reduce((sum, file) => sum + selectedPages(file), 0)
    const widths = quotaWidths(projected)
    document.querySelector('#view').innerHTML = `
      <section class="page submit-page reveal-page">
        ${pageHeader(
          '提交打印',
          `访问 ${stats.visit_count} 次 · 累计打印 ${stats.print_total_pages} 页`,
          `<button id="submit-files" class="primary-button" type="button" ${uploads.some((file) => file.status === 'ready') ? '' : 'disabled'}>${icon('send')}<span>${requiresApproval(projected) ? '提交审批' : '提交打印'}</span></button>`,
        )}
        <div class="submit-layout">
          <label id="dropzone" class="dropzone submit-dropzone">
            <span class="dropzone-icon">${icon('upload', 48)}</span>
            <strong>拖拽文件到此处</strong>
            <span>或点击选择文件</span>
            <small>支持 PDF、Word、Excel、PPT、图片、TXT，可多选</small>
            <input id="file-input" type="file" accept=".pdf,.doc,.docx,.xls,.xlsx,.ppt,.pptx,.jpg,.jpeg,.png,.bmp,.txt" multiple hidden />
          </label>
          <aside class="submission-sidebar">
            <div class="quota-progress-card ${requiresApproval(projected) ? 'danger' : ''}">
              <div class="quota-progress-heading"><span>今日额度</span><strong>${quota.remaining}/${quota.limit} 页</strong></div>
              <svg class="quota-track" viewBox="0 0 100 12" preserveAspectRatio="none" role="progressbar" aria-valuenow="${quota.remaining}" aria-valuemin="0" aria-valuemax="${quota.limit}">
                <defs>
                  <linearGradient id="quota-green-gradient" x1="0" x2="1">
                    <stop offset="0" stop-color="#2b9c55"></stop>
                    <stop offset="1" stop-color="#45b96f"></stop>
                  </linearGradient>
                  <linearGradient id="quota-pending-gradient" x1="0" x2="1">
                    <stop offset="0" stop-color="#f0b429"></stop>
                    <stop offset="1" stop-color="#f7c948"></stop>
                  </linearGradient>
                </defs>
                <rect width="100" height="12" fill="#dfe5ec"></rect>
                <rect class="quota-green-segment" width="${widths.green}" height="12" fill="url(#quota-green-gradient)"></rect>
                <rect class="quota-pending-segment" x="${widths.green}" width="${widths.pending}" height="12" fill="url(#quota-pending-gradient)"></rect>
              </svg>
              <div class="quota-legend">
                ${quotaLegendMarkup(projected)}
              </div>
            </div>
            <div class="upload-list">
              ${
                uploads
                  .map(uploadCardMarkup)
                  .join('') || '<p class="empty-upload-list">尚未添加文件</p>'
              }
            </div>
          </aside>
        </div>
      </section>`
    const input = document.querySelector('#file-input')
    input.addEventListener('change', () => {
      const files = [...input.files]
      input.value = ''
      uploadFiles(files)
    })
    const dropzone = document.querySelector('#dropzone')
    for (const name of ['dragenter', 'dragover']) {
      dropzone.addEventListener(name, (event) => {
        event.preventDefault()
        dropzone.classList.add('dragging')
      })
    }
    for (const name of ['dragleave', 'drop']) {
      dropzone.addEventListener(name, (event) => {
        event.preventDefault()
        dropzone.classList.remove('dragging')
      })
    }
    dropzone.addEventListener('drop', (event) => uploadFiles([...event.dataTransfer.files]))
    document.querySelectorAll('.upload-card').forEach(bindUploadCard)
    document.querySelector('#submit-files').addEventListener('click', async () => {
      const button = document.querySelector('#submit-files')
      const approvalRequired = requiresApproval(projectedPages())
      submitting = true
      button.disabled = true
      try {
        const result = await api('/print/submit', {
          method: 'POST',
          body: {
            files: uploads
              .filter((file) => file.status === 'ready')
              .map(({ temp_id, odd_even, custom_range }) => ({
                temp_id,
                odd_even,
                page_range: odd_even === 'custom' ? custom_range : undefined,
              })),
          },
        })
        notify(
          approvalRequired
            ? `已提交 ${result.tasks.length} 个审批任务，请等待管理员审核`
            : `已提交 ${result.tasks.length} 个任务`,
          'success',
        )
        location.hash = '#/queue'
      } catch (error) {
        notify(error.message, 'error')
        submitting = false
        button.disabled = false
      }
    })
  }

  async function uploadFiles(files) {
    if (files.length === 0) return
    const pendingUploads = files.map((file) => ({
      client_id: `pending-${Date.now()}-${++localUploadSequence}`,
      original_name: file.name,
      status: 'uploading',
      odd_even: 'all',
      custom_range: '',
      source_file: file,
    }))
    uploads.push(...pendingUploads)
    appendUploadCards(pendingUploads)

    await Promise.all(
      pendingUploads.map(async (pendingUpload) => {
        const data = new FormData()
        data.append('files', pendingUpload.source_file)
        try {
          const result = await api('/print/upload', { method: 'POST', body: data })
          const uploadedFile = result.files[0]
          if (!uploadedFile) throw new Error('服务器未返回上传结果')
          Object.assign(pendingUpload, uploadedFile, {
            status: 'ready',
            source_file: null,
          })
        } catch (error) {
          pendingUpload.status = 'failed'
          pendingUpload.error = error.message
          pendingUpload.source_file = null
          notify(`${pendingUpload.original_name}：${error.message}`, 'error')
        }
        replaceUploadCard(pendingUpload)
      }),
    )
  }
  renderPage()
  state.cleanup.push(() => {
    if (quotaAnimationFrame !== null) cancelAnimationFrame(quotaAnimationFrame)
    window.removeEventListener('keydown', closePreviewOnEscape)
    closePreview(true)
  })
}

const statusLabels = {
  pending_review: '待审核',
  queued: '排队中',
  spooling: '提交中',
  printing: '打印中',
  uncertain: '待人工确认',
  done: '已完成',
  cancelled: '已取消',
}

function printerStatusDisplay(printer, paused) {
  const raw = String(printer.status || '').toLowerCase()
  if (paused) return { label: 'Paused', text: '队列已暂停', tone: 'paused' }
  if (!printer.available || raw.includes('unavailable') || raw.includes('offline')) {
    return {
      label: 'Offline',
      text: raw.includes('offline') ? '打印机未连接' : '未检测到可用打印机',
      tone: 'offline',
    }
  }
  if (
    printer.blocked ||
    raw.includes('error') ||
    raw.includes('paper') ||
    raw.includes('jam')
  ) {
    return { label: 'Error', text: '打印机需要处理', tone: 'error' }
  }
  if (raw.includes('printing')) {
    return { label: 'Printing', text: '正在打印', tone: 'printing' }
  }
  if (raw.includes('initializing') || raw.includes('processing') || raw.includes('busy')) {
    return { label: 'Running', text: '打印机处理中', tone: 'running' }
  }
  return { label: 'Ready', text: '打印机就绪', tone: 'ready' }
}

async function renderQueue(silent = false) {
  const params = new URLSearchParams({
    page: state.queue.page,
    per_page: 30,
    mine_only: state.queue.mineOnly,
  })
  if (state.queue.studentId && !state.queue.mineOnly) params.set('student_id', state.queue.studentId)
  const data = await api(`/queue?${params}`)
  const isAdmin = state.user.role === 'admin'
  const printer = data.printer || {}
  const printerDisplay = printerStatusDisplay(printer, data.paused)
  document.querySelector('#view').innerHTML = `
    <section class="page queue-page ${silent ? '' : 'reveal-page'}">
      ${pageHeader(
        '打印队列',
        '当前队列与近一年记录',
        `<form id="queue-filter" class="button-row queue-toolbar">
          <div class="filter-search-control">
            <input id="queue-search" value="${escapeHtml(state.queue.studentId)}" placeholder="按学号筛选" ${state.queue.mineOnly ? 'disabled' : ''} />
            <button class="icon-button" type="submit" title="查询" aria-label="查询">${icon('search')}</button>
          </div>
          <label class="check-control"><input id="mine-only" type="checkbox" ${state.queue.mineOnly ? 'checked' : ''} /><span>只看我的打印</span></label>
          ${isAdmin ? `<button id="toggle-queue" class="${data.paused ? 'primary-button' : 'ghost-button'} queue-admin-button" type="button">${icon(data.paused ? 'play' : 'pause')}<span>${data.paused ? '继续队列' : '暂停队列'}</span></button>` : ''}
        </form>`,
      )}
      <section class="printer-card">
        <div class="printer-identity">
          <span class="printer-state-pill ${printerDisplay.tone}">${printerDisplay.label}</span>
          <div>
            <strong>${printerDisplay.text}</strong>
            <span>${escapeHtml(printer.queue_name || '正在读取打印机名称')}</span>
          </div>
        </div>
        <span class="printer-raw-status">驱动状态：${escapeHtml(printer.status || '-')}</span>
      </section>
      ${printer.blocked ? `<div class="alert-banner danger">打印已暂停：${escapeHtml((printer.blocking_reasons || []).join('；') || '打印机需要人工处理')}。问题解决后队列会自动继续。</div>` : ''}
      ${
        printer.warnings?.length && (!printer.toner_alert_acknowledged || !isAdmin)
          ? `<div class="alert-banner warning"><span>${escapeHtml(printer.warnings.join('；'))}</span>${isAdmin ? '<button id="ack-toner" class="ghost-button">确认提示</button>' : ''}</div>`
          : ''
      }
      <div class="task-list">
        ${
          data.tasks
            .map((task) => {
              const previewUrl = localApiUrl(task.preview_url)
              const sourceUrl = localApiUrl(task.source_url)
              const statusClass = Object.prototype.hasOwnProperty.call(statusLabels, task.status)
                ? task.status
                : 'unknown'
              return `
            <article class="task-card queue-task-card ${task.mine ? 'mine' : ''}" data-task="${task.id}">
              <div class="task-main">
                <div class="task-top"><div class="task-number"><strong>#${task.id}</strong>${task.mine ? '<span class="mine-badge">我的打印</span>' : ''}</div><span class="status-pill ${statusClass}">${statusLabels[task.status] || escapeHtml(task.status)}</span></div>
                <h3>${escapeHtml(task.file_name || '打印任务')}</h3>
                <p>${task.page_count} 页 · ${escapeHtml(pageSelectionLabel(task.odd_even))}${task.owner_name ? ` · ${escapeHtml(task.owner_name)}` : ''}</p>
                <p class="task-time">提交：${escapeHtml(task.submitted_at)}${task.completed_at ? ` · 结束：${escapeHtml(task.completed_at)}` : ''}</p>
                ${task.review_reason ? `<p class="danger-text">${escapeHtml(task.review_reason)}</p>` : ''}
                ${isAdmin && task.status_detail ? `<p class="task-time">${escapeHtml(task.status_detail)}</p>` : ''}
              </div>
              <div class="queue-task-actions">
                ${previewUrl ? `<a class="ghost-button" target="_blank" rel="noopener" href="${escapeHtml(previewUrl)}">${icon('eye')}<span>预览</span></a>` : ''}
                ${sourceUrl ? `<a class="ghost-button" target="_blank" rel="noopener" href="${escapeHtml(sourceUrl)}">${icon('download')}<span>下载</span></a>` : ''}
                ${(task.mine && ['queued', 'pending_review'].includes(task.status)) || (isAdmin && ['queued', 'pending_review', 'uncertain'].includes(task.status)) ? `<button class="ghost-button danger-text cancel-task">${icon('x')}<span>取消任务</span></button>` : ''}
              </div>
            </article>`
            })
            .join('') || '<p class="empty-state">没有符合条件的打印记录</p>'
        }
      </div>
      ${
        data.total > data.per_page
          ? `<footer class="pagination-bar"><span>第 ${data.page} 页</span><div class="button-row"><button id="queue-prev" class="icon-button" title="上一页" aria-label="上一页" ${data.page <= 1 ? 'disabled' : ''}>${icon('left')}</button><button id="queue-next" class="icon-button" title="下一页" aria-label="下一页" ${data.page * data.per_page >= data.total ? 'disabled' : ''}>${icon('right')}</button></div></footer>`
          : ''
      }
    </section>`
  document.querySelector('#mine-only').addEventListener('change', (event) => {
    state.queue.mineOnly = event.target.checked
    state.queue.page = 1
    renderQueue(true)
  })
  document.querySelector('#queue-filter').addEventListener('submit', (event) => {
    event.preventDefault()
    state.queue.studentId = document.querySelector('#queue-search').value.trim()
    state.queue.page = 1
    renderQueue(true)
  })
  document.querySelector('#queue-prev')?.addEventListener('click', () => {
    state.queue.page--
    renderQueue(true)
  })
  document.querySelector('#queue-next')?.addEventListener('click', () => {
    state.queue.page++
    renderQueue(true)
  })
  document.querySelector('#toggle-queue')?.addEventListener('click', async () => {
    try {
      await api(`/admin/queue/${data.paused ? 'resume' : 'pause'}`, { method: 'POST' })
      notify(data.paused ? '打印队列已恢复' : '打印队列已暂停', 'success')
      await renderQueue(true)
    } catch (error) {
      notify(error.message, 'error')
    }
  })
  document.querySelector('#ack-toner')?.addEventListener('click', async () => {
    await api('/admin/printer/ack-toner', { method: 'POST' })
    await renderQueue(true)
  })
  document.querySelectorAll('.cancel-task').forEach((button) =>
    button.addEventListener('click', async () => {
      const id = button.closest('[data-task]').dataset.task
      const task = data.tasks.find((item) => String(item.id) === id)
      const warning =
        task.status === 'uncertain'
          ? '请先在系统打印队列确认该作业未提交。确认后取消此任务？'
          : '确认取消此打印任务？'
      if (!confirm(warning)) return
      try {
        await api(isAdmin ? `/admin/tasks/${id}` : `/print/tasks/${id}`, {
          method: 'DELETE',
          body: isAdmin ? { reason: task.status === 'uncertain' ? '管理员确认提交失败' : null } : undefined,
        })
        await renderQueue(true)
      } catch (error) {
        notify(error.message, 'error')
      }
    }),
  )
  if (!silent) startQueueUpdates()
}

function startQueueUpdates() {
  const timer = setInterval(() => {
    if (routeName() === 'queue') renderQueue(true).catch(() => {})
  }, 60000)
  let socket
  let debounce
  try {
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
    socket = new WebSocket(`${protocol}//${location.host}/api/ws/queue`)
    socket.addEventListener('message', () => {
      clearTimeout(debounce)
      debounce = setTimeout(() => {
        if (routeName() === 'queue') renderQueue(true).catch(() => {})
      }, 250)
    })
  } catch {}
  state.cleanup.push(() => {
    clearInterval(timer)
    clearTimeout(debounce)
    socket?.close()
  })
}

async function renderSettings() {
  const contact = state.user.must_change_password ? {} : await api('/user/admin-contact')
  document.querySelector('#view').innerHTML = `
    <section class="page narrow-page reveal-page">
      ${pageHeader('个人设置', contact.qq ? `管理员 QQ：${escapeHtml(contact.qq)}` : '维护联系方式和登录密码')}
      ${state.user.must_change_password ? `<div class="alert-banner warning">首次登录必须先设置至少 ${state.minPasswordLength} 个字符的新密码。</div>` : ''}
      ${
        state.user.must_change_password
          ? ''
          : `<form id="profile-form" class="panel form-grid">
              <h2>联系方式</h2>
              <label>QQ<input name="qq" value="${escapeHtml(state.user.qq || '')}" /></label>
              <label>手机号<input name="phone" value="${escapeHtml(state.user.phone || '')}" /></label>
              <button class="primary-button">${icon('save')}<span>保存联系方式</span></button>
            </form>`
      }
      <form id="password-form" class="panel form-grid">
        <h2>修改密码</h2>
        ${passwordField('新密码', 'new_password', 'new-password', state.minPasswordLength)}
        ${passwordField('确认密码', 'confirm_password', 'new-password', state.minPasswordLength)}
        <button class="primary-button">${icon('save')}<span>修改并重新登录</span></button>
      </form>
    </section>`
  setupPasswordFields(document.querySelector('#password-form'))
  document.querySelector('#profile-form')?.addEventListener('submit', async (event) => {
    event.preventDefault()
    try {
      await api('/user/profile', {
        method: 'POST',
        body: { qq: formValue(event.currentTarget, 'qq'), phone: formValue(event.currentTarget, 'phone') },
      })
      setSession(await api('/auth/me'))
      notify('联系方式已保存', 'success')
    } catch (error) {
      notify(error.message, 'error')
    }
  })
  document.querySelector('#password-form').addEventListener('submit', async (event) => {
    event.preventDefault()
    try {
      await api('/auth/change-password', {
        method: 'POST',
        body: {
          new_password: rawFormValue(event.currentTarget, 'new_password'),
          confirm_password: rawFormValue(event.currentTarget, 'confirm_password'),
        },
      })
      state.user = null
      notify('密码已修改，请重新登录', 'success')
      renderLogin()
    } catch (error) {
      notify(error.message, 'error')
    }
  })
}

async function renderUsers() {
  const params = new URLSearchParams({ page: state.users.page, per_page: 50 })
  if (state.users.q) params.set('q', state.users.q)
  const data = await api(`/admin/users?${params}`)
  document.querySelector('#view').innerHTML = `
    <section class="page reveal-page">
      ${pageHeader(
        '用户管理',
        `共 ${data.total} 个账号`,
        `<form id="user-filter" class="button-row user-toolbar">
          <div class="filter-search-control">
            <input name="q" value="${escapeHtml(state.users.q)}" placeholder="按学号筛选" />
            <button class="icon-button" type="submit" title="查询" aria-label="查询">${icon('search')}</button>
          </div>
          <button id="add-user" class="ghost-button" type="button">${icon('plus')}<span>添加用户</span></button>
          <button id="import-users" class="ghost-button" type="button">${icon('upload')}<span>批量导入</span></button>
          <a class="ghost-button" href="/api/admin/stats.csv" target="_blank">${icon('download')}<span>导出统计</span></a>
        </form>`,
      )}
      <div class="table-scroll">
        <table class="data-table user-table">
          <thead><tr><th>学号</th><th>角色</th><th class="user-centered">状态</th><th class="user-centered">QQ</th><th class="user-centered">手机号</th><th class="user-centered">累计页数</th><th class="user-centered">任务数</th><th>操作</th></tr></thead>
          <tbody>${data.items
            .map(
              (user) => `<tr data-user="${user.id}">
                <td><strong>${escapeHtml(user.student_id)}</strong></td>
                <td>${user.role === 'admin' ? '管理员' : '用户'}</td>
                <td class="user-centered"><span class="status-pill ${user.status === 'banned' ? 'user-banned' : user.status === 'unused' ? 'user-unused' : 'user-normal'}">${user.status === 'banned' ? '封禁中' : user.status === 'unused' ? '未使用' : '正常'}</span></td>
                <td class="user-centered">${escapeHtml(user.qq || '-')}</td>
                <td class="user-centered">${escapeHtml(user.phone || '-')}</td>
                <td class="user-stat-value">${user.total_pages}</td>
                <td class="user-stat-value">${user.total_tasks}</td>
                <td class="row-actions">
                  <button class="icon-button reset-user" title="重置密码" aria-label="重置密码">${icon('key')}</button>
                  ${
                    user.id === state.user.id
                      ? ''
                      : `<button class="icon-button toggle-user" title="${user.status === 'banned' ? '解封' : '封禁'}" aria-label="${user.status === 'banned' ? '解封' : '封禁'}">${icon(user.status === 'banned' ? 'check' : 'ban')}</button><button class="icon-button danger-button delete-user" title="删除用户" aria-label="删除用户">${icon('trash')}</button>`
                  }
                </td>
              </tr>`,
            )
            .join('')}</tbody>
        </table>
      </div>
      ${
        data.total > data.per_page
          ? `<footer class="pagination-bar"><span>第 ${data.page} 页</span><div class="button-row"><button id="users-prev" class="icon-button" title="上一页" aria-label="上一页" ${data.page <= 1 ? 'disabled' : ''}>${icon('left')}</button><button id="users-next" class="icon-button" title="下一页" aria-label="下一页" ${data.page * data.per_page >= data.total ? 'disabled' : ''}>${icon('right')}</button></div></footer>`
          : ''
      }
    </section>`
  document.querySelector('#user-filter').addEventListener('submit', (event) => {
    event.preventDefault()
    state.users.q = formValue(event.currentTarget, 'q')
    state.users.page = 1
    renderUsers()
  })
  document.querySelector('#add-user').addEventListener('click', () => {
    openActionDialog({
      title: '添加用户',
      message: '新账号的初始密码与学号相同，首次登录后需要修改密码。',
      confirmText: '确认添加',
      inputLabel: '学号',
      inputRequired: true,
      onConfirm: async (studentId) => {
        await api('/admin/users', { method: 'POST', body: { student_id: studentId } })
        notify(`账号 ${studentId} 已创建，初始密码与学号相同`, 'success')
        await renderUsers()
      },
    })
  })
  document
    .querySelector('#import-users')
    .addEventListener('click', () => openImportUsersDialog(renderUsers))
  document.querySelectorAll('[data-user]').forEach((row) => {
    const user = data.items.find((item) => String(item.id) === row.dataset.user)
    row.querySelector('.reset-user').addEventListener('click', () => {
      openActionDialog({
        title: '重置密码',
        message: `将 ${user.student_id} 的密码重置为其学号，并撤销该账号的所有登录。`,
        confirmText: '确认重置',
        onConfirm: async () => {
          await userAction(`/admin/users/${user.id}/reset-password`, 'POST', {})
          notify(`${user.student_id} 的密码已重置为学号`, 'success')
        },
      })
    })
    row.querySelector('.toggle-user')?.addEventListener('click', () => {
      const unbanning = user.status === 'banned'
      openActionDialog({
        title: unbanning ? '解除封禁' : '封禁用户',
        message: unbanning
          ? `解除 ${user.student_id} 的封禁，允许该账号重新登录。`
          : `封禁 ${user.student_id} 后，该账号将立即退出且无法登录。`,
        confirmText: unbanning ? '确认解封' : '确认封禁',
        danger: !unbanning,
        onConfirm: async () => {
          await userAction(`/admin/users/${user.id}/status`, 'POST', {
            status: unbanning ? 'normal' : 'banned',
          })
          notify(`账号 ${user.student_id} 已${unbanning ? '解封' : '封禁'}`, 'success')
        },
      })
    })
    row.querySelector('.delete-user')?.addEventListener('click', () => {
      openActionDialog({
        title: '删除用户',
        message: `将永久删除 ${user.student_id} 及其打印记录和文件，此操作无法撤销。`,
        confirmText: '确认删除',
        danger: true,
        onConfirm: async () => {
          await userAction(`/admin/users/${user.id}`, 'DELETE')
          notify(`账号 ${user.student_id} 已删除`, 'success')
        },
      })
    })
  })
  document.querySelector('#users-prev')?.addEventListener('click', () => {
    state.users.page--
    renderUsers()
  })
  document.querySelector('#users-next')?.addEventListener('click', () => {
    state.users.page++
    renderUsers()
  })

  async function userAction(path, method, body) {
    await api(path, { method, body })
    await renderUsers()
  }
}

async function renderReview() {
  const tasks = await api('/admin/review')
  document.querySelector('#view').innerHTML = `
    <section class="page review-page reveal-page">
      ${pageHeader('审核中心', `${tasks.length} 个待审核任务`)}
      <div class="review-list ${tasks.length ? '' : 'empty'}">
        ${
          tasks
            .map(
              (task) => `<article class="review-item" data-task="${task.id}">
                <div class="review-item-main">
                  <div class="task-top"><strong>#${task.id} · ${escapeHtml(task.owner_name)}</strong><span class="status-pill pending_review">待审核</span></div>
                  <h3>${escapeHtml(task.file_name)}</h3>
                  <p>${task.page_count} 页 · ${escapeHtml(pageSelectionLabel(task.odd_even))}</p>
                </div>
                <div class="button-row review-actions"><button class="primary-button approve">${icon('check')}<span>同意</span></button><button class="ghost-button danger-text reject">${icon('x')}<span>拒绝</span></button></div>
              </article>`,
            )
            .join('') || '<p class="review-empty-state">当前没有待审核任务</p>'
        }
      </div>
    </section>`
  document.querySelectorAll('[data-task]').forEach((card) => {
    const id = card.dataset.task
    card.querySelector('.approve').addEventListener('click', () => reviewAction(id, 'approve'))
    card.querySelector('.reject').addEventListener('click', () => {
      const reason = prompt('拒绝原因（可留空）')
      if (reason !== null) reviewAction(id, 'reject', { reason: reason || null })
    })
  })
  async function reviewAction(id, action, body) {
    try {
      await api(`/admin/review/${id}/${action}`, { method: 'POST', body })
      await renderReview()
    } catch (error) {
      notify(error.message, 'error')
    }
  }
}

async function renderSystem() {
  const config = await api('/admin/config')
  document.querySelector('#view').innerHTML = `
    <section class="page narrow-page reveal-page">
      ${pageHeader('系统设置', '')}
      <form id="system-form" class="panel form-grid">
        <label>每日页数限额<input name="daily_limit" type="number" min="0" value="${escapeHtml(config.daily_limit)}" required /></label>
        <label>接任管理员学号（不转让则留空）<input name="new_admin" /></label>
        <button class="primary-button">${icon('save')}<span>保存设置</span></button>
      </form>
    </section>`
  document.querySelector('#system-form').addEventListener('submit', async (event) => {
    event.preventDefault()
    try {
      await api('/admin/config', {
        method: 'PUT',
        body: { key: 'daily_limit', value: formValue(event.currentTarget, 'daily_limit') },
      })
      const newAdmin = formValue(event.currentTarget, 'new_admin')
      if (newAdmin) {
        if (!confirm(`确认把管理员转让给 ${newAdmin}？当前账号将变为普通用户。`)) return
        await api('/admin/transfer', { method: 'POST', body: { new_admin_student_id: newAdmin } })
        setSession(await api('/auth/me'))
        location.hash = '#/submit'
      }
      notify('设置已保存', 'success')
    } catch (error) {
      notify(error.message, 'error')
    }
  })
}

window.addEventListener('hashchange', () => {
  updateNavHighlight()
  routeNavigation = routeNavigation
    .then(transitionRoute)
    .catch((error) => notify(error.message, 'error'))
})
window.addEventListener('error', (event) => {
  const message = event.error?.message || event.message || '页面资源加载失败'
  notify(message, 'error')
})
window.addEventListener('unhandledrejection', (event) => {
  event.preventDefault()
  const message = event.reason?.message || String(event.reason || '发生未知错误')
  notify(message, 'error')
})
restoreSession()
