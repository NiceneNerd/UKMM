class Modal extends Element {
  componentDidMount() {
    const theme = document.getRootNode().getAttribute("theme");
    const modal = Window.all.find((win) => win.parent != null);
    modal.document.head.innerHTML = "<style src='this://app/styles/modal.css' />";
    modal.document.getRootNode().setAttribute("theme", theme);
  }
  render(props, kids) {
    return <div #body>{kids}</div>;
  }
}

export default Modal;
