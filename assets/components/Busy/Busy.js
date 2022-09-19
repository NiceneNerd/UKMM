export const Busy = ({ text }) => (
  <div #Busy styleset={__DIR__ + "Busy.css#Busy"}>
    <div class="vspacer"></div>
    <p>Processing…</p>
    <p>{text}</p>
    <progress />
    <div class="vspacer"></div>
  </div>
);
