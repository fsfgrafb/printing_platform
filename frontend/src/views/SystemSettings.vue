<script setup>
import { onMounted, ref } from 'vue'
import { Save } from '@lucide/vue'
import { useRouter } from 'vue-router'
import { api, unwrapError } from '../api'
import { refreshSession } from '../session'

const config = ref({
  daily_limit: '50',
  queue_paused: false
})
const transferStudentId = ref('')
const saving = ref(false)
const loaded = ref(false)
const message = ref('')
const error = ref('')
const router = useRouter()

onMounted(load)

async function load() {
  try {
    const { data } = await api.get('/admin/config')
    config.value = data
    error.value = ''
  } catch (err) {
    error.value = unwrapError(err)
  } finally {
    loaded.value = true
  }
}

async function save() {
  saving.value = true
  message.value = ''
  error.value = ''
  try {
    await api.put('/admin/config', { key: 'daily_limit', value: String(config.value.daily_limit) })
    if (transferStudentId.value.trim()) {
      await api.post('/admin/transfer', { new_admin_student_id: transferStudentId.value.trim() })
      await refreshSession()
      message.value = '已保存，管理员已转让'
      await router.replace('/queue')
      return
    }
    message.value = '保存成功'
    await load()
  } catch (err) {
    error.value = unwrapError(err)
  } finally {
    saving.value = false
  }
}

</script>

<template>
  <section class="page narrow-page">
    <header class="page-header">
      <div>
        <h1>系统设置</h1>
      </div>
    </header>

    <p v-if="!loaded" class="loading-state">正在加载系统设置</p>

    <section v-if="loaded" class="panel form-grid">
      <label>
        每日限额
        <input v-model.trim="config.daily_limit" type="number" min="0" />
      </label>
      <label>
        转让管理员
        <input v-model.trim="transferStudentId" placeholder="新管理员学号，留空则不转让" />
      </label>
      <button class="primary-button" type="button" :disabled="saving" @click="save">
        <Save :size="18" />
        <span>{{ saving ? '保存中' : '保存' }}</span>
      </button>
    </section>

    <p v-if="message" class="ok-text">{{ message }}</p>
    <p v-if="error" class="error-text">{{ error }}</p>
  </section>
</template>
