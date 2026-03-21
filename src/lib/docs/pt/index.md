# Documentacao do Reasonance

## 1. Introducao

### O que e o Reasonance
Reasonance e um IDE de desktop leve projetado para vibecoders — desenvolvedores que trabalham com assistentes de codificacao baseados em IA. Ele fornece um ambiente limpo e focado com terminais LLM integrados, um editor de codigo e gerenciamento de arquivos.

### Requisitos do Sistema
- Linux (KDE Plasma recomendado), macOS ou Windows
- Pelo menos uma ferramenta CLI de LLM instalada (Claude Code, Ollama, etc.)

### Instalacao
Baixe a versao mais recente da pagina GitHub Releases. No Linux, instale o pacote .deb ou .AppImage.

## 2. Interface

### Layout
Reasonance utiliza um layout de tres paineis:
- **Painel esquerdo**: Arvore de arquivos para navegar seu projeto
- **Painel central**: Editor de codigo com destaque de sintaxe
- **Painel direito**: Terminal LLM para codificacao assistida por IA

### Barra de Menu
Acesse todas as funcionalidades pela barra de menu:
- **Arquivo**: Abrir pastas, gerenciar arquivos, projetos recentes
- **Editar**: Desfazer, refazer, area de transferencia, buscar
- **Exibir**: Tema, legibilidade, visibilidade dos paineis
- **Terminal**: Criar terminais LLM, modo YOLO
- **Git**: Status, commit, push, pull, log
- **Ajuda**: Documentacao, atalhos de teclado

### Barra de Status
A barra de status inferior mostra:
- Nome do app e quantidade de LLMs detectados
- Info da sessao do terminal ativo (contexto %, modelo, temporizador de reset, mensagens)
- Info do arquivo ativo (nome, linguagem, codificacao)
- Indicador de modo YOLO (barra vermelha quando ativo)

### Atalhos de Teclado
| Atalho | Acao |
|--------|------|
| Ctrl+P | Busca rapida de arquivos |
| Ctrl+Shift+F | Buscar em arquivos |
| Ctrl+S | Salvar arquivo |
| Ctrl+, | Abrir configuracoes |
| F1 | Abrir documentacao |

## 3. Gerenciamento de Arquivos

### Abrir um Projeto
Use **Arquivo > Abrir Pasta** ou clique em "Abrir Pasta" na tela de boas-vindas. Projetos recentes sao listados para acesso rapido.

### Navegar Arquivos
Clique nos arquivos na arvore de arquivos para abri-los. Clique com botao direito para acoes do menu de contexto. Use Ctrl+P para busca rapida de arquivo por nome.

### Editar Arquivos
Arquivos abrem em modo somente leitura por padrao. Clique em "Somente leitura" para alternar o modo de edicao. Alteracoes sao rastreadas com copias sombra para deteccao de diferencas.

### Busca
- **Ctrl+P**: Buscar arquivos por nome
- **Ctrl+Shift+F**: Buscar no conteudo dos arquivos (grep)

## 4. Terminal LLM

### Iniciar um LLM
Clique no botao **+** no painel de terminal para ver os LLMs disponiveis. Reasonance detecta automaticamente ferramentas CLI instaladas (Claude Code, Ollama, etc.).

### Multiplas Instancias
Execute multiplas sessoes LLM simultaneamente. Cada instancia tem sua propria aba. Alterne entre instancias usando a barra de abas.

### Modo YOLO
Ative o modo YOLO pela barra de ferramentas ou **Terminal > Modo YOLO**. Isso passa o flag --dangerously-skip-permissions para o Claude Code, permitindo que ele execute sem prompts de confirmacao. A barra de status fica vermelha como aviso.

### Rastreamento de Contexto
A barra de status exibe o uso da janela de contexto em tempo real, analisado a partir da saida do LLM, incluindo:
- Porcentagem de uso da sessao com barra visual
- Nome do modelo ativo
- Mensagens restantes
- Temporizador de contagem regressiva para reset

## 5. Integracao Git

Acesse comandos Git pelo menu **Git**. Comandos sao executados no terminal ativo:
- **Status**: Mostrar status da arvore de trabalho
- **Commit**: Iniciar um commit (digite sua mensagem)
- **Push**: Push para o remoto
- **Pull**: Pull do remoto
- **Log**: Mostrar historico de commits recentes

## 6. Configuracoes

Abra as configuracoes com **Ctrl+,** ou o icone de engrenagem.

### Tema
Escolha entre Claro, Escuro ou Sistema (segue a preferencia do SO). No KDE/Wayland, o modo Sistema usa deteccao nativa com fallback para Escuro.

### Idioma
Selecione entre 9 idiomas: English, Italiano, Deutsch, Espanol, Francais, Portugues, Zhongwen, Hindi, Al-Arabiya. Arabe habilita layout RTL.

### Fonte e Legibilidade
- Familia e tamanho de fonte personalizaveis
- Modo de Legibilidade Aprimorada: texto maior, espacamento aumentado, otimizado para acessibilidade

### Configuracao de LLM
LLMs sao detectados automaticamente no primeiro inicio. Configuracao manual via arquivo de configuracao TOML para configuracoes avancadas.

## 7. Solucao de Problemas

### LLMs Nao Detectados
- Certifique-se de que a ferramenta CLI de LLM esta instalada e no seu PATH
- Tente **Terminal > Detectar LLMs** para re-escanear
- Verifique o arquivo de configuracao para configuracao manual

### Renderizacao Borrada no Linux
Reasonance inclui uma correcao para escalonamento fracionario no KDE/Wayland (WebKitGTK). Se a renderizacao ainda estiver borrada, verifique suas configuracoes de escalonamento de tela.

### Tema Nao Muda
Se o tema nao responde a mudancas do sistema, tente configura-lo explicitamente para Claro ou Escuro nas Configuracoes, depois volte para Sistema.

### FAQ
**P: Posso usar multiplos LLMs ao mesmo tempo?**
R: Sim, cada LLM tem sua propria aba. Clique em + para adicionar mais instancias.

**P: Como configuro um LLM personalizado?**
R: Edite o arquivo de configuracao TOML em ~/.config/reasonance/config.toml

**P: O modo YOLO funciona com todos os LLMs?**
R: O modo YOLO esta atualmente otimizado para o Claude Code. Outros LLMs podem ter mecanismos de confirmacao diferentes.
