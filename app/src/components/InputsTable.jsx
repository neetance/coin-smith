function InputsTable({ inputs }) {

    return (
        <div className="card">

            <h3>Selected Inputs (UTXOs)</h3>

            <table>

                <thead>
                    <tr>
                        <th>TxID</th>
                        <th>Vout</th>
                        <th>Value</th>
                        <th>Script</th>
                        <th>Address</th>
                    </tr>
                </thead>

                <tbody>

                    {inputs.map((i, idx) => (
                        <tr key={idx}>

                            <td>{i.txid.slice(0, 10)}...</td>

                            <td>{i.vout}</td>

                            <td>{i.value_sats} sats</td>

                            <td>
                                <span className="badge badge-script">
                                    {i.script_type}
                                </span>
                            </td>

                            <td>{i.address || "-"}</td>

                        </tr>
                    ))}

                </tbody>

            </table>

        </div>
    );
}

export default InputsTable;