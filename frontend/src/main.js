import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import { configureUnauthorizedHandler } from './api'
import { session } from './session'
import './styles.css'

configureUnauthorizedHandler(() => {
  session.user = null
  session.ready = true
  if (router.currentRoute.value.path !== '/login') {
    router.replace('/login')
  }
})

createApp(App).use(router).mount('#app')
