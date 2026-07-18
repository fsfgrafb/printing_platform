<script setup>
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { Ban, CircleCheck, Download, KeyRound, Plus, Search, Send, Trash2, Upload, X } from '@lucide/vue'
import { api, unwrapError } from '../api'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { showError, showNotice, showSuccess } from '../notification'
import { refreshSession, session } from '../session'

const users = ref([])
const total = ref(0)
const file = ref(null)
const pendingAction = ref(null)
const transferStudentId = ref('')
const showAddUser = ref(false)
const showBatchImport = ref(false)
const newStudentId = ref('')
const keyword = ref('')
const roleFilter = ref('')
const statusFilter = ref('')
const busy = ref(false)
const loaded = ref(false)
const router = useRouter()
const importExtensions = new Set(['xlsx', 'xls', 'xlsm', 'csv', 'txt'])

onMounted(load)

async function load() {
  loaded.value = false
  try {
    const { data } = await api.get('/admin/users', {
      params: {
        page: 1,
        per_page: 200,
        q: keyword.value.trim() || undefined,
        role: roleFilter.value || undefined,
        status: statusFilter.value || undefined
      }
    })
    users.value = data.items
    total.value = data.total
    loaded.value = true
    return true
  } catch (err) {
    showError(unwrapError(err), {
      title: '用户列表加载失败',
      confirmText: '重试',
      onConfirm: load
    })
    return false
  }
}

async function importUsers() {
  if (!file.value || busy.value) return
  busy.value = true
  try {
    const formData = new FormData()
    formData.append('file', file.value)
    const { data } = await api.post('/admin/users/import', formData)
    file.value = null
    showBatchImport.value = false
    if (await load()) {
      showSuccess(`已新增 ${data.created.length} 人，跳过 ${data.skipped.length} 人。`, '导入完成')
    }
  } catch (err) {
    showError(unwrapError(err), { title: '用户导入失败' })
  } finally {
    busy.value = false
  }
}

function selectImportFile(event) {
  const selected = event.target.files?.[0] || null
  event.target.value = ''
  if (!selected) return
  const extension = selected.name.split('.').pop()?.toLowerCase()
  if (!extension || !importExtensions.has(extension)) {
    file.value = null
    showError('仅支持 XLSX、XLS、XLSM、CSV 和 TXT 文件。', { title: '文件格式不支持' })
    return
  }
  file.value = selected
}

function openBatchImport() {
  file.value = null
  showBatchImport.value = true
}

function closeBatchImport() {
  if (busy.value) return
  file.value = null
  showBatchImport.value = false
}

function exportCsv() {
  window.open('/api/admin/stats.csv', '_blank')
}

async function createUser() {
  if (busy.value) return
  const studentId = newStudentId.value.trim()
  if (!studentId) return
  busy.value = true
  try {
    await api.post('/admin/users', { student_id: studentId })
    newStudentId.value = ''
    showAddUser.value = false
    if (await load()) {
      showSuccess(`账号 ${studentId} 已创建，默认密码为学号。`)
    }
  } catch (err) {
    showError(unwrapError(err), { title: '新增用户失败' })
  } finally {
    busy.value = false
  }
}

function requestAction(type, user) {
  transferStudentId.value = ''
  pendingAction.value = { type, user }
}

async function confirmAction() {
  if (!pendingAction.value || busy.value) return
  const { type, user } = pendingAction.value
  busy.value = true
  try {
    if (type === 'reset') {
      await api.post(`/admin/users/${user.id}/reset-password`, {})
      if (user.id === session.user?.id) {
        await refreshSession()
        pendingAction.value = null
        showNotice({
          title: '密码已重置',
          message: '当前账号密码已重置为学号，请先修改密码。',
          onConfirm: () => router.replace('/settings')
        })
        return
      }
    } else if (type === 'delete') {
      await api.delete(`/admin/users/${user.id}`)
    } else if (type === 'ban' || type === 'unban') {
      await api.post(`/admin/users/${user.id}/status`, {
        status: type === 'ban' ? 'banned' : 'normal'
      })
    } else if (type === 'transfer') {
      await api.post('/admin/transfer', { new_admin_student_id: transferStudentId.value.trim() })
      await refreshSession()
      pendingAction.value = null
      showNotice({
        title: '管理员已转让',
        message: `管理员权限已转让给 ${transferStudentId.value.trim()}。`,
        onConfirm: () => router.replace('/queue')
      })
      return
    }
    pendingAction.value = null
    if (await load()) {
      showSuccess(
        type === 'reset'
          ? `${user.student_id} 的密码已重置为学号。`
          : type === 'delete'
            ? `账号 ${user.student_id} 已删除。`
            : type === 'ban'
              ? `账号 ${user.student_id} 已封禁。`
              : `账号 ${user.student_id} 已解封。`
      )
    }
  } catch (err) {
    showError(unwrapError(err))
  } finally {
    busy.value = false
  }
}

function roleLabel(user) {
  return user.role === 'admin' ? '管理员' : '用户'
}

function statusLabel(status) {
  return {
    normal: '正常',
    banned: '封禁中',
    unused: '未使用'
  }[status] || status
}

function resetFilters() {
  keyword.value = ''
  roleFilter.value = ''
  statusFilter.value = ''
  load()
}
</script>

<template>
  <section v-if="!loaded" class="page page-loading-shell">
    <p class="loading-state">正在加载用户</p>
  </section>

  <section v-else class="page reveal-page">
    <header class="page-header">
      <div>
        <h1>用户管理</h1>
        <p>共 {{ total }} 个账号</p>
      </div>
      <div class="button-row">
        <button class="ghost-button" type="button" @click="showAddUser = !showAddUser">
          <Plus :size="18" />
          <span>新增用户</span>
        </button>
        <button class="ghost-button" type="button" @click="openBatchImport">
          <Upload :size="18" />
          <span>批量创建用户</span>
        </button>
        <button class="ghost-button" type="button" @click="exportCsv">
          <Download :size="18" />
          <span>导出统计</span>
        </button>
      </div>
    </header>

    <form v-if="showAddUser" class="panel form-grid" @submit.prevent="createUser">
      <label>
        学号
        <input v-model.trim="newStudentId" autocomplete="off" />
      </label>
      <div class="button-row">
        <button class="primary-button" type="submit" :disabled="busy || !newStudentId.trim()">
          <Plus :size="18" />
          <span>新增用户</span>
        </button>
        <button class="ghost-button" type="button" :disabled="busy" @click="showAddUser = false">取消</button>
      </div>
    </form>

    <form class="panel user-filter-bar" @submit.prevent="load">
      <label class="filter-search">
        <span>搜索用户</span>
        <input v-model="keyword" placeholder="学号、手机号或 QQ" autocomplete="off" />
      </label>
      <label>
        <span>角色</span>
        <select v-model="roleFilter">
          <option value="">全部角色</option>
          <option value="admin">管理员</option>
          <option value="user">用户</option>
        </select>
      </label>
      <label>
        <span>状态</span>
        <select v-model="statusFilter">
          <option value="">全部状态</option>
          <option value="normal">正常</option>
          <option value="banned">封禁中</option>
          <option value="unused">未使用</option>
        </select>
      </label>
      <div class="button-row filter-actions">
        <button class="primary-button" type="submit">
          <Search :size="18" />
          <span>筛选</span>
        </button>
        <button class="ghost-button" type="button" @click="resetFilters">重置</button>
      </div>
    </form>

    <div class="table-scroll">
      <table class="data-table user-table">
        <thead>
          <tr><th>学号</th><th>角色</th><th>状态</th><th>手机号</th><th>QQ</th><th>累计页数</th><th>完成任务</th><th>注册时间</th><th>最后登录</th><th></th></tr>
        </thead>
        <tbody>
          <tr v-for="user in users" :key="user.id">
            <td>{{ user.student_id }}</td>
            <td>{{ roleLabel(user) }}</td>
            <td><span class="status-pill" :class="`user-${user.status}`">{{ statusLabel(user.status) }}</span></td>
            <td>{{ user.phone || '-' }}</td>
            <td>{{ user.qq || '-' }}</td>
            <td>{{ user.total_pages }}</td>
            <td>{{ user.total_tasks }}</td>
            <td>{{ user.created_at || '-' }}</td>
            <td>{{ user.last_login_at || '-' }}</td>
            <td class="row-actions">
              <button class="icon-button" type="button" title="重置为学号密码" @click="requestAction('reset', user)">
                <KeyRound :size="18" />
              </button>
              <button
                v-if="user.id !== session.user?.id && user.status !== 'banned'"
                class="icon-button danger-button"
                type="button"
                title="封禁用户"
                @click="requestAction('ban', user)"
              >
                <Ban :size="18" />
              </button>
              <button
                v-else-if="user.id !== session.user?.id"
                class="icon-button"
                type="button"
                title="解除封禁"
                @click="requestAction('unban', user)"
              >
                <CircleCheck :size="18" />
              </button>
              <button
                v-if="user.id === session.user?.id"
                class="icon-button"
                type="button"
                title="转让管理员"
                @click="requestAction('transfer', user)"
              >
                <Send :size="18" />
              </button>
              <button v-else class="icon-button danger-button" type="button" title="删除用户" @click="requestAction('delete', user)">
                <Trash2 :size="18" />
              </button>
            </td>
          </tr>
        </tbody>
      </table>
      <p v-if="users.length === 0" class="empty-state">没有符合条件的用户</p>
    </div>

    <ConfirmDialog
      v-if="pendingAction"
      :title="pendingAction.type === 'reset'
        ? '重置密码'
        : pendingAction.type === 'delete'
          ? '删除用户'
          : pendingAction.type === 'ban'
            ? '封禁用户'
            : pendingAction.type === 'unban'
              ? '解除封禁'
              : '转让管理员'"
      :message="pendingAction.type === 'reset'
        ? `将 ${pendingAction.user.student_id} 的密码重置为其学号，并要求下次登录修改。`
        : pendingAction.type === 'delete'
          ? `将永久删除 ${pendingAction.user.student_id} 及其打印记录和文件。`
          : pendingAction.type === 'ban'
            ? `封禁 ${pendingAction.user.student_id} 后，该账号将立即退出且无法登录。`
            : pendingAction.type === 'unban'
              ? `解除 ${pendingAction.user.student_id} 的封禁，允许该账号重新登录。`
              : '只能转让给已有用户。转让后，当前账号将变为普通用户。'"
      :confirm-text="pendingAction.type === 'reset'
        ? '确认重置'
        : pendingAction.type === 'delete'
          ? '确认删除'
          : pendingAction.type === 'ban'
            ? '确认封禁'
            : pendingAction.type === 'unban'
              ? '确认解封'
              : '确认转让'"
      :danger="pendingAction.type === 'delete' || pendingAction.type === 'ban'"
      :input-label="pendingAction.type === 'transfer' ? '新管理员学号' : ''"
      :input-required="pendingAction.type === 'transfer'"
      :input-value="transferStudentId"
      :busy="busy"
      @update:input-value="transferStudentId = $event"
      @cancel="pendingAction = null"
      @confirm="confirmAction"
    />

    <div v-if="showBatchImport" class="dialog-backdrop" role="presentation" @click.self="closeBatchImport">
      <form class="confirm-dialog batch-import-dialog" role="dialog" aria-modal="true" aria-label="批量创建用户" @submit.prevent="importUsers">
        <header>
          <strong>批量创建用户</strong>
          <button class="icon-button" type="button" title="关闭" :disabled="busy" @click="closeBatchImport">
            <X :size="18" />
          </button>
        </header>
        <div class="batch-format-note">
          <p>请按以下格式准备文件，识别到的学号将用于创建账号，默认密码与学号相同：</p>
          <ul>
            <li>表格文件：支持 XLSX、XLS、XLSM 和 CSV，只识别第一列。</li>
            <li>文本文件：支持 TXT，每行填写一个学号。</li>
          </ul>
        </div>
        <label class="file-button batch-file-button">
          <Upload :size="18" />
          <span>{{ file ? file.name : '选择文件' }}</span>
          <input type="file" accept=".xlsx,.xls,.xlsm,.csv,.txt" hidden @change="selectImportFile" />
        </label>
        <footer>
          <button class="ghost-button" type="button" :disabled="busy" @click="closeBatchImport">取消</button>
          <button class="primary-button" type="submit" :disabled="!file || busy">
            {{ busy ? '创建中…' : '开始创建' }}
          </button>
        </footer>
      </form>
    </div>
  </section>
</template>
