export class Toolbar extends Element {
  render(props, kids) {
    return <div styleset={__DIR__ + "Toolbar.css#Toolbar"}>{kids}</div>;
  }
}
