import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import { Playground } from './ui/Playground.tsx'

const root = document.getElementById('root')
if (!root) throw new Error('root element is missing')

createRoot(root).render(
  <StrictMode>
    <Playground />
  </StrictMode>,
)
