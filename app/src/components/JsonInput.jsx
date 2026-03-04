import { useState } from "react";

function JsonInput({ setResult }) {

    const [text, setText] = useState("");

    const submit = async () => {

        try {
            const parsed = JSON.parse(text);

            const res = await fetch("http://localhost:8080/api/build", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json"
                },
                body: JSON.stringify(parsed)
            });

            const data = await res.json();

            setResult(data);

        } catch {
            alert("Invalid JSON");
        }
    };

    return (
        <div className="card">

            <h3>Paste Fixture JSON</h3>

            <textarea
                rows={10}
                value={text}
                onChange={(e) => setText(e.target.value)}
            />

            <br />

            <button onClick={submit}>Build Transaction</button>

        </div>
    );
}

export default JsonInput;