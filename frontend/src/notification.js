import { reactive } from 'vue'

export const notification = reactive({
  visible: false,
  title: '',
  message: '',
  confirmText: '确定',
  danger: false,
  onConfirm: null
})

const pendingNotices = []

export function showNotice({
  title = '提示',
  message = '',
  confirmText = '确定',
  danger = false,
  onConfirm = null
}) {
  const notice = { title, message, confirmText, danger, onConfirm }
  if (notification.visible) {
    pendingNotices.push(notice)
    return
  }
  displayNotice(notice)
}

export function showError(message, options = {}) {
  showNotice({
    title: options.title || '操作失败',
    message,
    confirmText: options.confirmText || '确定',
    danger: true,
    onConfirm: options.onConfirm || null
  })
}

export function showSuccess(message, title = '操作成功') {
  showNotice({ title, message })
}

export function confirmNotice() {
  const onConfirm = notification.onConfirm
  notification.visible = false
  notification.onConfirm = null
  if (onConfirm) onConfirm()
  if (!notification.visible && pendingNotices.length) {
    displayNotice(pendingNotices.shift())
  }
}

function displayNotice(notice) {
  notification.title = notice.title
  notification.message = notice.message
  notification.confirmText = notice.confirmText
  notification.danger = notice.danger
  notification.onConfirm = notice.onConfirm
  notification.visible = true
}
