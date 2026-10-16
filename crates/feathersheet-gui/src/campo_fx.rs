//! Adaptador fino do TextInput do Iced 0.13 para observar o caret real.
 //! Não redesenha nem reimplementa o editor; delega todos os eventos ao Iced.
use iced::{
    advanced::{self, Widget, widget::tree::{self,Tree}, layout, renderer},
    event::{self,Event},
    keyboard, mouse, widget::text_input,
    Element, Length, Rectangle, Renderer, Size, Theme,
};
use crate::app::Msg;

pub struct CampoFx<'a> {
    input: text_input::TextInput<'a,Msg>,
    texto: &'a str,
}
impl<'a> CampoFx<'a> {
    pub fn new(input:text_input::TextInput<'a,Msg>,texto:&'a str)->Self {
        Self {input,texto}
    }
}
impl Widget<Msg,Theme,Renderer> for CampoFx<'_> {
    fn tag(&self)->tree::Tag {Widget::tag(&self.input)}
    fn state(&self)->tree::State {Widget::state(&self.input)}
    fn diff(&self,tree:&mut Tree){Widget::diff(&self.input,tree)}
    fn size(&self)->Size<Length>{Widget::size(&self.input)}
    fn layout(&self,tree:&mut Tree,renderer:&Renderer,limits:&layout::Limits)->layout::Node {
        Widget::layout(&self.input,tree,renderer,limits)
    }
    fn operate(&self,tree:&mut Tree,layout:advanced::Layout<'_>,
        renderer:&Renderer,op:&mut dyn advanced::widget::Operation) {
        Widget::operate(&self.input,tree,layout,renderer,op);
    }
    fn on_event(&mut self,tree:&mut Tree,evento:Event,layout:advanced::Layout<'_>,
        cursor:mouse::Cursor,renderer:&Renderer,clipboard:&mut dyn advanced::Clipboard,
        shell:&mut advanced::Shell<'_,Msg>,viewport:&Rectangle)->event::Status {
        // Enter com Alt é uma quebra de linha, não um submit. O evento
        // global de teclado inicia a edição multilinha na célula ativa.
        if matches!(&evento, Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Enter), modifiers, ..
        }) if modifiers.alt()) {
            let estado = tree.state.downcast_ref::<text_input::State<
                <Renderer as advanced::text::Renderer>::Paragraph>>();
            if estado.is_focused() { return event::Status::Captured; }
        }
        let observar=matches!(&evento,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left) |
                         mouse::Event::ButtonReleased(mouse::Button::Left)));
        let dentro=cursor.is_over(layout.bounds());
        let resultado=Widget::on_event(&mut self.input,tree,evento,layout,cursor,renderer,clipboard,shell,viewport);
        if observar && dentro {
            // Estado real do TextInput depois do click/arrasto do mouse.
            let estado=tree.state.downcast_ref::<text_input::State<
                <Renderer as advanced::text::Renderer>::Paragraph>>();
            let value=text_input::Value::new(self.texto);
            match estado.cursor().state(&value) {
                text_input::cursor::State::Index(i) =>
                    shell.publish(Msg::PosicionarCursorFx(i)),
                text_input::cursor::State::Selection {start,end} =>
                    shell.publish(Msg::SelecionarTextoFx(start.min(end),start.max(end))),
            };
        }
        resultado
    }
    fn mouse_interaction(&self,tree:&Tree,layout:advanced::Layout<'_>,
        cursor:mouse::Cursor,viewport:&Rectangle,renderer:&Renderer)->mouse::Interaction {
        Widget::mouse_interaction(&self.input,tree,layout,cursor,viewport,renderer)
    }
    fn draw(&self,tree:&Tree,renderer:&mut Renderer,theme:&Theme,
        estilo:&renderer::Style,layout:advanced::Layout<'_>,cursor:mouse::Cursor,
        viewport:&Rectangle) {
        Widget::draw(&self.input,tree,renderer,theme,estilo,layout,cursor,viewport);
    }
}
/// Remove o foco do campo fx, devolvendo os atalhos de teclado à grade.
///
/// O Iced 0.13 não expõe text_input::unfocus(id), por isso fazemos uma
/// operação que desfoca SOMENTE este input e preserva outras janelas/campos.
pub fn desfocar() -> iced::Task<Msg> {
    struct DesfocarCampoFx;

    impl advanced::widget::Operation<Msg> for DesfocarCampoFx {
        fn container(
            &mut self,
            _id: Option<&advanced::widget::Id>,
            _bounds: Rectangle,
            operar_filhos: &mut dyn FnMut(&mut dyn advanced::widget::Operation<Msg>),
        ) {
            operar_filhos(self);
        }

        fn focusable(
            &mut self,
            estado: &mut dyn advanced::widget::operation::Focusable,
            id: Option<&advanced::widget::Id>,
        ) {
            if id == Some(&advanced::widget::Id::new("feathersheet.fx")) {
                estado.unfocus();
            }
        }
    }

    advanced::widget::operate(DesfocarCampoFx)
}

/// Transfere o foco do campo fx para o primeiro editor multilinha situado
/// após ele na árvore. Iced 0.13 não oferece text_editor::focus(id).
pub fn focar_multilinha() -> iced::Task<Msg> {
    struct FocarMultilinha { dentro: bool, focou: bool }
    impl advanced::widget::Operation<Msg> for FocarMultilinha {
        fn container(&mut self, id: Option<&advanced::widget::Id>, _bounds: Rectangle,
            filhos: &mut dyn FnMut(&mut dyn advanced::widget::Operation<Msg>)) {
            let anterior = self.dentro;
            if id == Some(&advanced::widget::Id::new("feathersheet.multilinha")) {
                self.dentro = true;
            }
            filhos(self);
            self.dentro = anterior;
        }
        fn focusable(&mut self, estado: &mut dyn advanced::widget::operation::Focusable,
            id: Option<&advanced::widget::Id>) {
            if id == Some(&advanced::widget::Id::new("feathersheet.fx")) {
                estado.unfocus();
            } else if self.dentro && !self.focou {
                estado.focus();
                self.focou = true;
            }
        }
    }
    advanced::widget::operate(FocarMultilinha { dentro: false, focou: false })
}

impl<'a> From<CampoFx<'a>> for Element<'a,Msg> {
    fn from(campo:CampoFx<'a>)->Self {Element::new(campo)}
}
