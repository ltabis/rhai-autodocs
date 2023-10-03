export const Highlight = ({ children, color }) => (
    <span
        style={{
            backgroundColor: color,
            borderRadius: '2px',
            color: '#000',
            padding: '0.2rem',
        }}>
        {children}
    </span>
);