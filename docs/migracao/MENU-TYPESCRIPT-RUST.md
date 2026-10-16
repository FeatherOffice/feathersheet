# FeatherSheet — comparativo do menu TypeScript × Rust/Iced

**Atualização 08/10/2026 — implementação parcial publicada na `main`; validação Cargo/UI pendente.**

## Problema corrigido

O TypeScript de `feathersheet/src/ui/barras/menus.ts` expunha **9 menus com
submenus de vários níveis** e ações como Colar especial, Ver, Imagem, Tabela,
Formatação condicional e ferramentas. A GUI Iced já carregava uma reprodução
desse catálogo de `dados/menus.json`, mas vários comandos já implementados
no Rust estavam **desabilitados no dispatcher** e a apresentação de duas
colunas ocupava espaço excessivo na área da planilha.

A migração preserva a nomenclatura e a estrutura. O catálogo atual contém
**147 entradas com ação** — as 145 do legado somadas às novas opções
Pintar linha/coluna e Somente formatação.

### Menu atualizado

Na parte superior, clique em **Menu**, à esquerda das abas da Ribbon.
A janela usa uma barra horizontal de categorias, com scroll horizontal quando
necessário:

**Arquivo · Editar · Ver · Inserir · Formatar · Dados · Macros · Ferramentas · Ajuda.**

Ao entrar em um submenu, a trilha aparece no cabeçalho
(ex.: `Formatar › Formatação condicional › Limpar regras`), com
**← Voltar** e **×**. A lista possui scroll vertical interno,
largura máxima de 500 px e altura limitada ao espaço da janela.
A edição da planilha e a barra de fórmulas não são deslocadas.

Itens sem ícone no TypeScript deixam de exibir um ícone artificial de tabela;
os demais usam ícones Tabler SVG incorporados pelo `feather-ui`.
Menus de estado **Linhas de grade**, **Pintar linha/coluna**,
**Barra de fórmulas**, **Tema** e **Zoom** indicam a opção atual.

## Ações que passaram a ser executadas

| Local | Ação agora ligada ao motor nativo |
|---|---|
| Editar → Colar especial | Somente valores; Somente formatação |
| Ver → Mostrar | Barra de fórmulas; Pintar linha/coluna |
| Inserir | Tabela; Imagem |
| Formatar | Limpar formatação; Cores alternadas; Ajustar largura |
| Formatar → Formatação condicional | Nova regra; Gerenciar regras; Limpar regras da seleção / da planilha (para opções equivalentes no Rust) |
| Ferramentas | Catálogo de funções abre a aba Inserir |
| Ajuda | Sobre aciona a tela de informações do FeatherSheet |
| Ribbon Página Inicial | Colar valores usa a mesma ação nativa do menu |

Essas ações complementam Novo, Abrir, Salvar, Salvar como, Exportar Excel,
Desfazer/Refazer, copiar/recortar/colar, Zoom, formatação de células,
inserção/exclusão de linhas/colunas, gráficos, ordenação de intervalo,
duplicatas, macros e monitor já disponíveis anteriormente.

**Limites deliberados:** apresentar uma entrada do TypeScript **não**
significa que sua lógica foi migrada. Exportar CSV/TSV/JSON, imprimir,
congelar linhas/colunas, filtro de dados, dividir texto, criar tabela dinâmica,
gerenciar nomes e itens de macros específicos permanecem **desabilitados
com rótulo “em migração”**, se ainda não possuem rotina Rust equivalente.
Não substituí esses comandos por mensagens vazias e não inventei
handlers. Não foi alterado o formato nativo `.fshz`.

## Arquivos

- `feathersheet/crates/feathersheet-gui/dados/menus.json` — árvore completa
  herdada do TypeScript + duas opções reais.
- `src/app/menu_nativo.rs` — catálogo tipado, nomes dos SVGs e testes.
- `src/app.rs` — apresentação compacta, estado, navegação e mensagens reais.
- `src/app/faixa_nativa.rs` — ativação do botão Colar valores.

## Como testar no Ubuntu (raiz do repositório)

```bash
git pull --ff-only origin main

cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml \
  menu_typescript_restaurado -j 2

cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml \
  menu_conecta_acoes_rust_existentes -j 2

cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml \
  todos_os_icones_do_catalogo_apontam_para_svg_tabler_existente -j 2

cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml \
  menu_legado_completo_tem_nove_grupos -j 2

cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml \
  --all-targets -j 2

cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

**Smoke manual:** clique Menu, percorra as nove categorias e submenus,
verifique breadcrumb e Voltar. Em Editar → Colar especial, cole valores e
formatos em células de teste; em Ver → Mostrar, alterne Pintar linha/coluna;
em Inserir, teste Imagem e Tabela; em Formatar, teste Limpar formatação e
Cores alternadas. Com a janela reduzida, confira scroll horizontal das
categorias e scroll vertical da lista sem sobrepor a barra de fórmulas.

**Sem aprovação fictícia:** o ambiente do agente não contém Cargo nem Rustc;
os testes Rust e os screenshots de aceitação visual ainda precisam rodar
no computador do mantenedor. A demanda de integração UI-005 permanece `[/]`.
