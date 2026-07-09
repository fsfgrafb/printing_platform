<script setup>
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { Printer } from '@lucide/vue'
import { api, unwrapError } from '../api'
import { session } from '../session'

const router = useRouter()
const studentId = ref('')
const password = ref('')
const error = ref('')
const loading = ref(false)

async function login() {
  error.value = ''
  loading.value = true
  try {
    const { data } = await api.post('/auth/login', {
      student_id: studentId.value,
      password: password.value
    })
    session.user = data.user
    router.replace(data.user.must_change_password ? '/settings' : '/submit')
  } catch (err) {
    error.value = unwrapError(err)
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <main class="login-screen">
    <section class="login-panel">
      <div class="login-brand">
        <Printer :size="32" />
        <div>
          <h1>ACM 实验室自助打印平台</h1>
          <p>内网打印队列与限额管理</p>
        </div>
      </div>

      <form class="form-grid" @submit.prevent="login">
        <label>
          学号
          <input v-model.trim="studentId" autocomplete="username" required />
        </label>
        <label>
          密码
          <input v-model="password" type="password" autocomplete="current-password" required />
        </label>
        <p v-if="error" class="error-text">{{ error }}</p>
        <button class="primary-button" type="submit" :disabled="loading">
          {{ loading ? '登录中' : '登录' }}
        </button>
      </form>
    </section>
  </main>
</template>
