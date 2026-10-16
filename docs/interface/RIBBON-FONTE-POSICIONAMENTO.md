# FeatherSheet — alinhamento do grupo Fonte na Ribbon (08/10/2026)

## Problema visual reproduzido por imagem

Na aba **Página Inicial**, controles de fonte apareciam desalinhados:
os botões para alterar o tamanho exibiam sinal de mais/menos duplicado
(ícone e texto), o seletor de tamanho ficava apertado, e a legenda
**Fonte** encostava na área da barra de rolagem horizontal.

## Correção realizada

- `feather-ui/src/componentes.rs`: nova função **`grupo_faixa_compacto`**,
  disponível para os três editores, mas aplicada **somente** ao grupo Fonte
  de FeatherSheet. O corpo recebe 68 px lógicos e o grupo 96 px,
  com espaço próprio para a legenda e separador; as demais seções da
  Ribbon continuam com o comportamento anterior.
- `feathersheet-gui/src/app/faixa_nativa.rs`: seleção de fonte ampliada
  para 148 px e de tamanho para 65 px; botões +/− agora são
  **só ícones com tooltips**, preservando `Msg::TamanhoFonteDelta`.
  Segunda linha usa quatro botões estilísticos, cores e bordas com
  espaçamento uniforme. Comandos de células, paletas e armazenamento
  do workbook não foram alterados.
- Testes Rust adicionados para validar que a altura da legenda
  cabe na faixa e que a criação do grupo Fonte não modifica o
  modelo (inclusive após selecionar a página inicial).

## COMO TESTAR NO MEU COMPUTADOR

Da raiz do repositório:

```bash
git pull --ff-only origin main
cargo test --manifest-path feather-ui/Cargo.toml grupo_compacto_reserva_legenda -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml ribbon_fonte_compacta -j 2
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml --all-targets -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

Verificação visual em **Página Inicial → Fonte**:

1. Fonte e tamanho aparecem na **primeira linha**, alinhados verticalmente.
2. **−** e **+** aparecem uma única vez (hover explica a ação).
3. Negrito, itálico, sublinhado, tachado, Texto, Fundo e Bordas aparecem
   na **segunda linha**, sem recortes ou sobreposições.
4. A legenda **Fonte** fica inteira e não se mistura à barra de rolagem.
5. Testar janela com largura mínima e zoom de interface/DPI diferente;
   a Ribbon pode rolar horizontalmente, mas seus rótulos devem permanecer
   separados e legíveis.
6. Confirmar que tamanho, cores, bordas e atalhos ainda aplicam alterações
   à seleção e continuam compatíveis com o documento salvo.

**Limite de validação:** esta execução publicou código e testes,
mas o ambiente não dispõe de `cargo`/`rustc` nem exibiu o FeatherSheet.
O aceite de compilação e captura visual permanece pendente.

## Correção adicional — barra de rolagem sobre a legenda Alinhamento

Uma segunda captura mostrou que o scrollbar horizontal ainda cruzava o
texto **Alinhamento**. O problema afetava todos os grupos que usavam
`grupo_faixa` com `Length::Fill` e legenda em altura dinâmica, não só Fonte.

**Solução aplicada na Ribbon do FeatherSheet:**

- `faixa_grupo` agora reutiliza `feather_ui::componentes::grupo_faixa_compacto`
  para **todas** as abas, garantindo legenda dentro da área de controles.
- Em `view_faixa`, as ferramentas continuam numa área de 100 px e existe
  um **rodapé de 22 px somente para o scrollbar horizontal**.
- Altura do viewport: 135 px; altura total da Ribbon: **141 px**.
  O scrollbar fica fisicamente abaixo de "Fonte", "Alinhamento" e demais
  títulos, inclusive quando a Ribbon precisa rolar lateralmente.
- A ancoragem da galeria de gráficos usa a **mesma constante**
  `faixa_nativa::ALTURA_FAIXA_NATIVA` para que a galeria continue abrindo
  **abaixo** da Ribbon após a correção.

**Regressão Rust adicionada:**

```bash
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml scroll_horizontal_da_ribbon_nao_ocupa_legendas -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml galeria_graficos -j 2
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml --all-targets -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

**Aceite visual:** em janela estreita, abrir **Página Inicial**, mover a
barra horizontal para revelar os grupos **Fonte**, **Alinhamento** e
**Número**, e garantir que a pista não passe sobre nenhuma legenda.
Repetir em **Inserir → Gráfico** para verificar que o popover não cobre
a Ribbon. Testes de compilação e GUI ainda **não executados** aqui.
