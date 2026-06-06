export function add(a, b) {
  return a + b;
}

export class Service {
  rotate(token) {
    return token + "!";
  }
}

export const make = () => {
  function inner(x) {
    return x;
  }
  return inner;
};

// JSX in a .js file — only the TSX grammar parses this.
export function Badge(props) {
  return <span className="badge">{props.label}</span>;
}
