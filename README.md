# LazySshRs

Um gerenciador de configuração SSH em Rust com interface TUI (Terminal User Interface).

## Sobre o Projeto

Este projeto foi inspirado na ferramenta [lazyssh](https://github.com/Adembc/lazyssh) e foi desenvolvido como um projeto de estudo da linguagem Rust, explorando conceitos como:

- Desenvolvimento de interfaces TUI com `ratatui`
- Gerenciamento de arquivos e configurações
- Parsing de arquivos SSH config
- Execução de processos externos
- Arquitetura modular em Rust

## Funcionalidades

### 🔍 **Visualização e Navegação**
- Interface TUI interativa e intuitiva
- Leitura automática de arquivos SSH config
- Suporte a arquivos `Include` organizados por pastas
- Navegação com setas entre hosts
- Visualização detalhada das configurações

### 🔌 **Conectividade**
- **Conexão SSH direta**: Pressione `Enter` para conectar
- **Teste de conectividade TCP**: Tecla `p` para ping na porta SSH
- Transição suave entre TUI e console SSH
- Retorno automático à interface após desconexão

### 📝 **Gerenciamento de Hosts**
- **Adicionar hosts**: Tecla `a` com formulário completo
- **Editar hosts**: Tecla `e` para modificar configurações existentes
- **Campos suportados**: Host, Hostname, User, Port, IdentityFile, LocalForward
- **Organização por pastas**: Hosts organizados em diferentes arquivos
- **Include automático**: Novos arquivos adicionados automaticamente ao config principal

### 🔎 **Busca Inteligente**
- **Busca fuzzy**: Tecla `/` para busca inteligente
- **Resultados em tempo real**: Filtragem conforme digitação
- **Ordenação por relevância**: Melhores matches primeiro
- **Navegação nos resultados**: Setas para navegar entre matches

### ⚙️ **Configuração**
- **Arquivo de configuração**: `~/.config/lazysshrs`
- **Workdir configurável**: Define pasta base para arquivos SSH
- **Padrão flexível**: Usa `~/.ssh/` por padrão, mas permite customização

## Como usar

### Instalação
```bash
git clone <repository-url>
cd lazysshrs
cargo build --release
```

### Execução
```bash
cargo run
```

### Controles

#### Navegação Principal
- `↑/↓`: Navegar entre hosts
- `Enter`: Conectar via SSH ao host selecionado
- `q`: Sair da aplicação

#### Gerenciamento
- `a`: Adicionar novo host
- `e`: Editar host selecionado
- `p`: Testar conectividade (ping TCP)
- `/`: Buscar hosts (busca fuzzy)

#### Formulários
- `Tab/Shift+Tab`: Navegar entre campos
- `Enter`: Confirmar/Avançar
- `Esc`: Cancelar/Voltar
- `Backspace`: Apagar caracteres

#### Busca
- `Digite`: Filtrar hosts
- `↑/↓`: Navegar nos resultados
- `Enter`: Selecionar host
- `Esc`: Cancelar busca

## Estrutura do Projeto

```
src/
├── main.rs           # Ponto de entrada
├── config.rs         # Configuração da aplicação
├── ssh_config.rs     # Parser de arquivos SSH config
├── tui.rs           # Interface TUI principal
├── form.rs          # Formulários para hosts
└── connectivity.rs   # Testes de conectividade e SSH
```

## Dependências

- `ratatui`: Interface TUI moderna
- `crossterm`: Controle multiplataforma do terminal
- `fuzzy-matcher`: Busca fuzzy inteligente
- `serde` + `toml`: Serialização e configuração
- `home`: Localização do diretório home

## Inspiração

Este projeto foi inspirado na excelente ferramenta [lazyssh](https://github.com/Adembc/lazyssh) e desenvolvido como um exercício de aprendizado da linguagem Rust, explorando suas capacidades para desenvolvimento de ferramentas de linha de comando com interfaces TUI.

## Licença

ISC License - Veja [LICENSE](LICENSE) para detalhes.