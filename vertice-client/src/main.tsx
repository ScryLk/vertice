import { createRoot } from 'react-dom/client';

import App from './App';
import './styles.css';

// Sem `StrictMode` de propósito. Em desenvolvimento ele monta o efeito duas
// vezes, o que abriria dois WebSockets para o mesmo assento — o servidor
// aguenta (a geração protege), mas o log fica mentindo sobre o que o jogador
// fez, e essa sujeira atrapalha exatamente o que esta tela existe para
// enxergar.
createRoot(document.getElementById('root')!).render(<App />);
