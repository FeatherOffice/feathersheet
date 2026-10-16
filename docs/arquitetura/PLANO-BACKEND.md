# FeatherSheet — plano de migração nativa

**Direção atual: 100% Rust com Iced, motor e interface.** Este é o terceiro
foco da suíte, depois de FeatherSlide e FeatherDocs. FeatherCAD vem depois,
se houver tempo. A migração será retomada quando o FeatherSheet receber o foco.

## Aproveitar o motor existente

Preservar os crates `feathersheet-core` e `feathersheet-io`, suas regras de
fórmulas, recálculo, armazenamento colunar, formatação e leitura/gravação.
Os status e nomes de módulos devem ser conferidos no código antes de iniciar
cada etapa; as porcentagens anteriores não medem a conclusão da interface Iced.

## Interface nativa

- Criar a aplicação Iced com janelas, menus, faixa de opções e preferências.
- Implementar a grade com desenho apenas das células visíveis, zoom, rolagem e seleção.
- Ligar edição, barra de fórmulas, comandos e desfazer/refazer ao motor Rust.
- Migrar painéis de formatação, gráficos, impressão, consulta e revisão.
- Reavaliar integração de macros sem duplicar regras do documento na interface.
- Validar comportamento e desempenho com as planilhas reais do usuário.

## Verificação

Usar Cargo para compilação, testes e análise dos crates alterados, conforme
[`../../RECEITA.md`](../../../RECEITA.md). A interface chama o motor diretamente.
O inventário de funcionalidades fica em [`ROADMAP.md`](../planejamento/ROADMAP.md), os
detalhes de arquitetura em [`ARQUITETURA.md`](ARQUITETURA.md), e as anotações
de trabalho em [`../CLAUDE.md`](../../CLAUDE.md).
