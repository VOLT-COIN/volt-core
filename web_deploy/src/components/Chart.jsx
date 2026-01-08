import React, { useEffect, useRef, useState } from 'react';
import { createChart } from 'lightweight-charts';
import axios from 'axios';

const API_URL = "http://localhost:6001/api";

const Chart = ({ pair }) => {
    const chartContainerRef = useRef();
    const [candles, setCandles] = useState([]);

    useEffect(() => {
        const fetchCandles = async () => {
            try {
                const res = await axios.post(API_URL, { command: "get_candles", token: pair });
                if (res.data.status === 'success') {
                    // Convert Backend Candle to Chart Format
                    // Backend: time (u64 epoch), open, high, low, close (u64 atomic)
                    // Chart: time (seconds), open, high, low, close (float)
                    const data = res.data.data.candles.map(c => ({
                        time: c.time,
                        open: c.open / 100000000,
                        high: c.high / 100000000,
                        low: c.low / 100000000,
                        close: c.close / 100000000,
                    }));

                    // Sort by time just in case
                    data.sort((a, b) => a.time - b.time);
                    setCandles(data);
                }
            } catch (e) {
                console.error("Chart fetch error", e);
            }
        };

        fetchCandles();
        const interval = setInterval(fetchCandles, 5000); // Live update
        return () => clearInterval(interval);
    }, [pair]);

    useEffect(() => {
        if (!chartContainerRef.current || candles.length === 0) return;

        const chart = createChart(chartContainerRef.current, {
            width: chartContainerRef.current.clientWidth,
            height: 400,
            layout: {
                backgroundColor: '#1E1E1E',
                textColor: '#DDD',
            },
            grid: {
                vertLines: { color: '#2B2B43' },
                horzLines: { color: '#2B2B43' },
            },
            timeScale: {
                timeVisible: true,
                secondsVisible: false,
            }
        });

        const candleSeries = chart.addCandlestickSeries({
            upColor: '#26a69a',
            downColor: '#ef5350',
            borderVisible: false,
            wickUpColor: '#26a69a',
            wickDownColor: '#ef5350',
        });

        candleSeries.setData(candles);
        chart.timeScale().fitContent();

        const handleResize = () => {
            chart.applyOptions({ width: chartContainerRef.current.clientWidth });
        };
        window.addEventListener('resize', handleResize);

        return () => {
            window.removeEventListener('resize', handleResize);
            chart.remove();
        };
    }, [candles]);

    return (
        <div className="chart-wrapper">
            <h3>{pair ? pair : "Select Pair"} Chart</h3>
            <div ref={chartContainerRef} className="chart-container" />
        </div>
    );
};

export default Chart;
