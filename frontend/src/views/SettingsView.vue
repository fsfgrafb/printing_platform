<script setup>
import { onMounted, ref } from 'vue'
import { Save } from '@lucide/vue'
import { api, unwrapError } from '../api'
import { refreshSession, session } from '../session'

const oldPassword = ref('')
const newPassword = ref('')
const qq = ref('')
const admin = ref({ student_id: '', qq: '' })
const message = ref('')
const error = ref('')

onMounted(async () => {
  qq.value = session.user?.qq || ''
  const { data } = await api.get('/user/admin-contact')
  admin.value = data
})

async function changePassword() {
  error.value = ''
  try {
    await api.post('/auth/change-password', {
      old_password: oldPassword.value,
      new_password: newPassword.value
    })
    oldPassword.value = ''
    newPassword.value = ''
    message.value = '密码已修改'
    await refreshSession()
  } catch (err) {
    error.value = unwrapError(err)
  }
}

async function saveProfile() {
  await api.post('/user/profile', { qq: qq.value })
  message.value = '资料已保存'
  await refreshSession()
}
</script>

<template>
  <section class="page narrow-page">
    <header class="page-header">
      <div>
        <h1>个人设置</h1>
        <p>管理员：{{ admin.student_id || '-' }} · QQ：{{ admin.qq || '-' }}</p>
      </div>
    </header>

    <section class="panel form-grid">
      <label>
        我的 QQ
        <input v-model.trim="qq" />
      </label>
      <button class="primary-button" type="button" @click="saveProfile">
        <Save :size="18" />
        <span>保存资料</span>
      </button>
    </section>

    <section class="panel form-grid">
      <label>
        当前密码
        <input v-model="oldPassword" type="password" />
      </label>
      <label>
        新密码
        <input v-model="newPassword" type="password" />
      </label>
      <button class="primary-button" type="button" @click="changePassword">
        <Save :size="18" />
        <span>修改密码</span>
      </button>
      <p v-if="message" class="ok-text">{{ message }}</p>
      <p v-if="error" class="error-text">{{ error }}</p>
    </section>
  </section>
</template>
