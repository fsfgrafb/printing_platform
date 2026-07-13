<script setup>
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { Save } from '@lucide/vue'
import { api, unwrapError } from '../api'
import { refreshSession, session } from '../session'
import ConfirmDialog from '../components/ConfirmDialog.vue'

const newPassword = ref('')
const confirmPassword = ref('')
const qq = ref('')
const admin = ref({ student_id: '', qq: '' })
const loaded = ref(false)
const message = ref('')
const error = ref('')
const showProfileDialog = ref(false)
const showPasswordDialog = ref(false)
const router = useRouter()

onMounted(async () => {
  try {
    qq.value = session.user?.qq || ''
    if (!session.user?.must_change_password) {
      const { data } = await api.get('/user/admin-contact')
      admin.value = data
    }
  } catch (err) {
    error.value = unwrapError(err)
  } finally {
    loaded.value = true
  }
})

async function changePassword() {
  error.value = ''
  try {
    if (newPassword.value !== confirmPassword.value) {
      error.value = '两次输入的新密码不一致'
      return
    }
    await api.post('/auth/change-password', {
      new_password: newPassword.value,
      confirm_password: confirmPassword.value
    })
    newPassword.value = ''
    confirmPassword.value = ''
    session.user = null
    showPasswordDialog.value = true
  } catch (err) {
    error.value = unwrapError(err)
  }
}

async function saveProfile() {
  error.value = ''
  try {
    await api.post('/user/profile', { qq: qq.value })
    await refreshSession()
    showProfileDialog.value = true
  } catch (err) {
    error.value = unwrapError(err)
  }
}
</script>

<template>
  <section class="page narrow-page">
    <header class="page-header">
      <div>
        <h1>个人设置</h1>
        <p v-if="loaded">管理员：{{ admin.student_id || '-' }} · QQ：{{ admin.qq || '-' }}</p>
      </div>
    </header>

    <p v-if="!loaded" class="loading-state">正在加载个人设置</p>

    <div v-if="loaded && session.user?.must_change_password" class="alert-banner warning">
      首次登录必须先修改默认密码，完成后才能使用打印功能。
    </div>

    <section v-if="loaded && !session.user?.must_change_password" class="panel form-grid">
      <label>
        我的 QQ
        <input v-model.trim="qq" name="qq_number" autocomplete="off" inputmode="numeric" />
      </label>
      <button class="primary-button" type="button" @click="saveProfile">
        <Save :size="18" />
        <span>保存资料</span>
      </button>
    </section>

    <section v-if="loaded" class="panel form-grid">
      <label>
        新密码
        <input v-model="newPassword" type="password" autocomplete="new-password" />
      </label>
      <label>
        确认密码
        <input v-model="confirmPassword" type="password" autocomplete="new-password" />
      </label>
      <button class="primary-button" type="button" @click="changePassword">
        <Save :size="18" />
        <span>修改密码</span>
      </button>
      <p v-if="message" class="ok-text">{{ message }}</p>
    </section>

    <ConfirmDialog
      v-if="showProfileDialog"
      title="保存成功"
      confirm-text="确定"
      :show-cancel="false"
      @confirm="showProfileDialog = false"
    />

    <ConfirmDialog
      v-if="showPasswordDialog"
      title="密码已修改"
      message="请使用新密码重新登录。"
      confirm-text="确定"
      :show-cancel="false"
      @confirm="router.replace('/login')"
    />

    <ConfirmDialog
      v-if="error"
      title="操作失败"
      :message="error"
      confirm-text="确定"
      :show-cancel="false"
      @confirm="error = ''"
    />
  </section>
</template>
