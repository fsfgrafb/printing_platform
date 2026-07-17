<script setup>
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { Save } from '@lucide/vue'
import { api, unwrapError } from '../api'
import PasswordField from '../components/PasswordField.vue'
import { showError, showNotice, showSuccess } from '../notification'
import { refreshSession, session } from '../session'

const newPassword = ref('')
const confirmPassword = ref('')
const qq = ref('')
const admin = ref({ student_id: '', qq: '' })
const loaded = ref(false)
const profileSaving = ref(false)
const passwordSaving = ref(false)
const router = useRouter()

onMounted(load)

async function load() {
  loaded.value = false
  try {
    qq.value = session.user?.qq || ''
    if (!session.user?.must_change_password) {
      const { data } = await api.get('/user/admin-contact')
      admin.value = data
    }
    loaded.value = true
  } catch (err) {
    showError(unwrapError(err), {
      title: '个人设置加载失败',
      confirmText: '重试',
      onConfirm: load
    })
  }
}

async function changePassword() {
  if (passwordSaving.value) return
  try {
    if (newPassword.value !== confirmPassword.value) {
      showError('两次输入的新密码不一致', { title: '无法修改密码' })
      return
    }
    passwordSaving.value = true
    await api.post('/auth/change-password', {
      new_password: newPassword.value,
      confirm_password: confirmPassword.value
    })
    newPassword.value = ''
    confirmPassword.value = ''
    session.user = null
    showNotice({
      title: '密码已修改',
      message: '请使用新密码重新登录。',
      onConfirm: () => router.replace('/login')
    })
  } catch (err) {
    showError(unwrapError(err), { title: '密码修改失败' })
  } finally {
    passwordSaving.value = false
  }
}

async function saveProfile() {
  if (profileSaving.value) return
  profileSaving.value = true
  try {
    await api.post('/user/profile', { qq: qq.value })
    await refreshSession()
    showSuccess('个人资料已更新。')
  } catch (err) {
    showError(unwrapError(err), { title: '资料保存失败' })
  } finally {
    profileSaving.value = false
  }
}
</script>

<template>
  <section v-if="!loaded" class="page page-loading-shell">
    <p class="loading-state">正在加载个人设置</p>
  </section>

  <section v-else class="page narrow-page reveal-page">
    <header class="page-header">
      <div>
        <h1>个人设置</h1>
        <p v-if="!session.user?.must_change_password">管理员：{{ admin.student_id || '-' }} · QQ：{{ admin.qq || '-' }}</p>
      </div>
    </header>

    <div v-if="session.user?.must_change_password" class="alert-banner warning">
      首次登录必须先修改默认密码，完成后才能使用打印功能。
    </div>

    <form v-if="!session.user?.must_change_password" class="panel form-grid" @submit.prevent="saveProfile">
      <label>
        我的 QQ
        <input v-model.trim="qq" name="qq_number" autocomplete="off" inputmode="numeric" />
      </label>
      <button class="primary-button" type="submit" :disabled="profileSaving">
        <Save :size="18" />
        <span>{{ profileSaving ? '保存中' : '保存资料' }}</span>
      </button>
    </form>

    <form class="panel form-grid" @submit.prevent="changePassword">
      <PasswordField v-model="newPassword" label="新密码" autocomplete="new-password" required />
      <PasswordField v-model="confirmPassword" label="确认密码" autocomplete="new-password" required />
      <button class="primary-button" type="submit" :disabled="passwordSaving">
        <Save :size="18" />
        <span>{{ passwordSaving ? '修改中' : '修改密码' }}</span>
      </button>
    </form>
  </section>
</template>
