<script setup>
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { api, unwrapError } from '../api'
import PasswordField from '../components/PasswordField.vue'
import { showError } from '../notification'
import { session } from '../session'

const router = useRouter()
const studentId = ref('')
const password = ref('')
const loading = ref(false)

async function login() {
  if (loading.value) return
  loading.value = true
  try {
    const { data } = await api.post('/auth/login', {
      student_id: studentId.value,
      password: password.value
    })
    session.user = data.user
    router.replace(data.user.must_change_password ? '/settings' : '/submit')
  } catch (err) {
    showError(unwrapError(err), { title: '登录失败' })
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <main class="login-screen">
    <section class="login-panel">
      <div class="login-brand">
        <img class="login-icon" src="/favicon.svg" alt="" />
        <div>
          <h1>ACM 实验室自助打印平台</h1>
        </div>
      </div>

      <form class="form-grid" @submit.prevent="login">
        <label>
          学号
          <input v-model.trim="studentId" autocomplete="username" required />
        </label>
        <PasswordField v-model="password" label="密码" autocomplete="current-password" required />
        <button class="primary-button" type="submit" :disabled="loading">
          {{ loading ? '登录中' : '登录' }}
        </button>
      </form>
    </section>
  </main>
</template>
