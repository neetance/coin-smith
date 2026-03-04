function OutputsTable({ outputs }) {

    return (
        <div className="card">

            <h3>Outputs</h3>

            <table>

                <thead>
                    <tr>
                        <th>#</th>
                        <th>Value</th>
                        <th>Script</th>
                        <th>Address</th>
                        <th>Type</th>
                    </tr>
                </thead>

                <tbody>

                    {outputs.map((o) => (

                        <tr key={o.n}>

                            <td>{o.n}</td>

                            <td>{o.value_sats} sats</td>

                            <td>
                                <span className="badge badge-script">
                                    {o.script_type}
                                </span>
                            </td>

                            <td>{o.address || "-"}</td>

                            <td>
                                {o.is_change ? (
                                    <span className="badge badge-change">
                                        change
                                    </span>
                                ) : "payment"}
                            </td>

                        </tr>

                    ))}

                </tbody>

            </table>

        </div>
    );
}

export default OutputsTable;