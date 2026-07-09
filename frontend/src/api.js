import axios from 'axios'

export const api = axios.create({
  baseURL: '/api',
  withCredentials: true
})

let unauthorizedHandler = null

export function configureUnauthorizedHandler(handler) {
  unauthorizedHandler = handler
}

api.interceptors.response.use(
  response => response,
  error => {
    if (error?.response?.status === 401 && unauthorizedHandler) {
      unauthorizedHandler()
    }
    return Promise.reject(error)
  }
)

export function unwrapError(error) {
  return error?.response?.data?.error || error?.message || '请求失败'
}
