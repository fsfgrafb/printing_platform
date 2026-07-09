import { reactive } from 'vue'
import { api } from './api'

export const session = reactive({
  user: null,
  ready: false
})

export async function refreshSession() {
  try {
    const { data } = await api.get('/auth/me')
    session.user = data.user
  } catch {
    session.user = null
  } finally {
    session.ready = true
  }
}

export async function signOut() {
  await api.post('/auth/logout')
  session.user = null
}
