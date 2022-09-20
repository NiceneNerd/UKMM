export const Spoiler = ({ label }, kids) => (
  <select type="tree" styleset={__DIR__ + "Spoiler.css#Spoiler"}>
    <option>
      <caption>{label}</caption>
      <option>{kids}</option>
    </option>
  </select>
);
