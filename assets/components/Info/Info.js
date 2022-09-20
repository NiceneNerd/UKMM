export const Info = (props, kids) => (
  <div class="info-box" styleset={__DIR__ + "Info.css#Info"}>
    {kids}
  </div>
);

export const Row = ({ key, val }) => (
  <div class="row">
    <div class="label">{key}</div>
    <div class="data">{val}</div>
  </div>
);

export const Long = ({ key, val, markdown, className }) => (
  <div class={"long " + (className ? className : "")}>
    <div class="label">{key}</div>
    <div class={"data " + (markdown && "md")}>{val}</div>
  </div>
);
